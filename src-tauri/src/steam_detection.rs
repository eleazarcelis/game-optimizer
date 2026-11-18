use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    os::windows::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};
use winreg::{RegKey, enums::*};

const REPARSE_POINT: u32 = 0x400;
const MAX_METADATA: u64 = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 500_000;
const MAX_CANDIDATES: usize = 2_000;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionTarget {
    Steam,
    Games,
    All,
}

#[derive(Serialize)]
pub struct DetectionResult {
    steam: Option<String>,
    games: Vec<DetectedGame>,
    issues: Vec<DetectionIssue>,
}

#[derive(Serialize)]
pub struct DetectedGame {
    app_id: String,
    name: String,
    candidates: Vec<String>,
}

#[derive(Serialize)]
pub struct DetectionIssue {
    kind: &'static str,
    path: Option<String>,
}

fn issue(issues: &mut Vec<DetectionIssue>, kind: &'static str, path: Option<&Path>, detail: &str) {
    if issues.len() >= 100 {
        if issues.len() == 100 {
            log::warn!("Steam detection: additional issues omitted");
            issues.push(DetectionIssue {
                kind: "issues_truncated",
                path: None,
            });
        }
        return;
    }
    log::warn!("Steam detection ({kind}): {detail}");
    issues.push(DetectionIssue {
        kind,
        path: path.map(|p| p.to_string_lossy().into_owned()),
    });
}

#[derive(Debug)]
enum Value {
    Text(String),
    Object(Vec<(String, Value)>),
}

#[derive(Debug)]
enum Token {
    Word(String),
    Open,
    Close,
}

fn tokens(input: &str) -> Result<Vec<Token>, String> {
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    let mut result = Vec::new();
    while let Some(ch) = chars.next() {
        match ch {
            c if c.is_whitespace() => {}
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '{' => result.push(Token::Open),
            '}' => result.push(Token::Close),
            '"' => {
                let mut value = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match chars.next() {
                            Some('\\') => value.push('\\'),
                            Some('"') => value.push('"'),
                            Some(other) => {
                                value.push('\\');
                                value.push(other);
                            }
                            None => return Err("Incomplete escape in Steam metadata".into()),
                        },
                        _ => value.push(c),
                    }
                }
                if !closed {
                    return Err("Unterminated string in Steam metadata".into());
                }
                result.push(Token::Word(value));
            }
            _ => {
                let mut value = String::from(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || next == '{' || next == '}' {
                        break;
                    }
                    value.push(chars.next().expect("peeked character"));
                }
                result.push(Token::Word(value));
            }
        }
    }
    Ok(result)
}

fn object(
    tokens: &[Token],
    index: &mut usize,
    nested: bool,
    depth: usize,
) -> Result<Vec<(String, Value)>, String> {
    if depth > 64 {
        return Err("Steam metadata nesting is too deep".into());
    }
    let mut result = Vec::new();
    while let Some(key) = tokens.get(*index) {
        *index += 1;
        let key = match key {
            Token::Close => {
                return if nested {
                    Ok(result)
                } else {
                    Err("Unexpected closing brace".into())
                };
            }
            Token::Open => return Err("Unexpected opening brace".into()),
            Token::Word(key) => key,
        };
        let next = tokens.get(*index).ok_or("Missing Steam metadata value")?;
        *index += 1;
        let value = match next {
            Token::Open => Value::Object(object(tokens, index, true, depth + 1)?),
            Token::Close => return Err("Missing Steam metadata value".into()),
            Token::Word(value) => Value::Text(value.clone()),
        };
        result.push((key.clone(), value));
    }
    if nested {
        Err("Missing closing brace".into())
    } else {
        Ok(result)
    }
}

fn parse(input: &str) -> Result<Vec<(String, Value)>, String> {
    object(&tokens(input)?, &mut 0, false, 0)
}

fn field<'a>(values: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    values
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| value)
}

fn text<'a>(values: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    match field(values, key)? {
        Value::Text(value) => Some(value),
        Value::Object(_) => None,
    }
}

fn read_metadata(path: &Path) -> Result<Vec<(String, Value)>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.file_attributes() & REPARSE_POINT != 0 {
        return Err("Steam metadata is not a regular file".into());
    }
    if metadata.len() > MAX_METADATA {
        return Err("Steam metadata file is too large".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_METADATA + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_METADATA {
        return Err("Steam metadata file is too large".into());
    }
    let content = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    parse(&content)
}

fn steam_path(issues: &mut Vec<DetectionIssue>) -> Option<PathBuf> {
    for (root, key, value) in [
        (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Valve\Steam",
            "InstallPath",
        ),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
    ] {
        let key = match RegKey::predef(root).open_subkey(key) {
            Ok(key) => key,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                issue(issues, "registry", None, &error.to_string());
                continue;
            }
        };
        let value: String = match key.get_value(value) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                issue(issues, "registry", None, &error.to_string());
                continue;
            }
        };
        let path = PathBuf::from(value.replace('/', "\\"));
        if path.is_absolute() && path.join("steam.exe").is_file() {
            return Some(path);
        }
        issue(
            issues,
            "invalid_steam",
            Some(&path),
            "Registered Steam executable is unavailable",
        );
    }
    None
}

fn libraries(steam: &Path, issues: &mut Vec<DetectionIssue>) -> Vec<PathBuf> {
    let mut libraries = vec![steam.to_owned()];
    let file = steam.join("steamapps").join("libraryfolders.vdf");
    let entries = match read_metadata(&file) {
        Ok(entries) => entries,
        Err(error) => {
            issue(issues, "library_metadata", Some(&file), &error);
            return libraries;
        }
    };
    let Some(Value::Object(folders)) = field(&entries, "libraryfolders") else {
        issue(
            issues,
            "library_metadata",
            Some(&file),
            "Missing libraryfolders object",
        );
        return libraries;
    };
    for (key, value) in folders {
        if !key.chars().all(|c| c.is_ascii_digit()) || key.is_empty() {
            continue;
        }
        let path = match value {
            Value::Text(path) => Some(path.as_str()),
            Value::Object(fields) => text(fields, "path"),
        };
        let Some(path) = path else {
            issue(
                issues,
                "library_metadata",
                Some(&file),
                "Missing library path",
            );
            continue;
        };
        let path = PathBuf::from(path.replace('/', "\\"));
        if !path.is_absolute() {
            issue(
                issues,
                "library_metadata",
                Some(&file),
                "Library path is not absolute",
            );
            continue;
        }
        if !libraries.iter().any(|existing| {
            existing
                .to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .eq_ignore_ascii_case(path.to_string_lossy().trim_end_matches(['\\', '/']))
        }) {
            libraries.push(path);
        }
    }
    libraries
}

fn executable_candidates(root: &Path, issues: &mut Vec<DetectionIssue>) -> Vec<String> {
    let mut pending = vec![root.to_owned()];
    let mut candidates = Vec::new();
    let mut inspected = 0usize;
    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                issue(issues, "directory", Some(&directory), &error.to_string());
                continue;
            }
        };
        for entry in entries {
            inspected += 1;
            if inspected > MAX_ENTRIES || candidates.len() >= MAX_CANDIDATES {
                issue(issues, "scan_limit", Some(root), "Game scan limit reached");
                candidates.sort_unstable_by_key(|path: &String| path.to_lowercase());
                return candidates;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    issue(issues, "directory", Some(&directory), &error.to_string());
                    continue;
                }
            };
            let metadata = match fs::symlink_metadata(entry.path()) {
                Ok(metadata) => metadata,
                Err(error) => {
                    issue(issues, "directory", Some(&entry.path()), &error.to_string());
                    continue;
                }
            };
            if metadata.file_attributes() & REPARSE_POINT != 0 {
                issue(
                    issues,
                    "link_skipped",
                    Some(&entry.path()),
                    "Skipped reparse point",
                );
            } else if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            {
                candidates.push(entry.path().to_string_lossy().into_owned());
            }
        }
    }
    candidates.sort_unstable_by_key(|path| path.to_lowercase());
    candidates
}

fn scan_games(libraries: &[PathBuf], issues: &mut Vec<DetectionIssue>) -> Vec<DetectedGame> {
    let mut games = Vec::new();
    for library in libraries {
        let steamapps = library.join("steamapps");
        let entries = match fs::read_dir(&steamapps) {
            Ok(entries) => entries,
            Err(error) => {
                issue(issues, "library", Some(&steamapps), &error.to_string());
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    issue(issues, "library", Some(&steamapps), &error.to_string());
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                continue;
            }
            let file = entry.path();
            let parsed = match read_metadata(&file) {
                Ok(parsed) => parsed,
                Err(error) => {
                    issue(issues, "manifest", Some(&file), &error);
                    continue;
                }
            };
            let Some(Value::Object(app)) = field(&parsed, "AppState") else {
                issue(issues, "manifest", Some(&file), "Missing AppState");
                continue;
            };
            let (Some(app_id), Some(name), Some(install_dir), Some(flags)) = (
                text(app, "appid"),
                text(app, "name"),
                text(app, "installdir"),
                text(app, "StateFlags").and_then(|value| value.parse::<u32>().ok()),
            ) else {
                issue(
                    issues,
                    "manifest",
                    Some(&file),
                    "Missing application metadata",
                );
                continue;
            };
            if flags & 4 == 0 {
                continue;
            }
            let mut parts = Path::new(install_dir).components();
            if !app_id.chars().all(|c| c.is_ascii_digit())
                || app_id.is_empty()
                || name.trim().is_empty()
                || !matches!(parts.next(), Some(Component::Normal(_)))
                || parts.next().is_some()
            {
                issue(
                    issues,
                    "manifest",
                    Some(&file),
                    "Invalid application identity or installation folder",
                );
                continue;
            }
            let root = steamapps.join("common").join(install_dir);
            match fs::symlink_metadata(&root) {
                Ok(metadata)
                    if metadata.is_dir() && metadata.file_attributes() & REPARSE_POINT == 0 => {}
                Ok(_) => {
                    issue(
                        issues,
                        "link_skipped",
                        Some(&root),
                        "Game installation is not a normal directory",
                    );
                    continue;
                }
                Err(error) => {
                    issue(issues, "directory", Some(&root), &error.to_string());
                    continue;
                }
            }
            games.push(DetectedGame {
                app_id: app_id.to_owned(),
                name: name.to_owned(),
                candidates: executable_candidates(&root, issues),
            });
        }
    }
    games.sort_unstable_by_key(|game| game.name.to_lowercase());
    games
}

pub fn detect(target: DetectionTarget) -> DetectionResult {
    let mut issues = Vec::new();
    let Some(root) = steam_path(&mut issues) else {
        issue(
            &mut issues,
            "steam_missing",
            None,
            "Steam installation was not found",
        );
        return DetectionResult {
            steam: None,
            games: Vec::new(),
            issues,
        };
    };
    let steam = if matches!(target, DetectionTarget::Steam | DetectionTarget::All) {
        Some(root.join("steam.exe").to_string_lossy().into_owned())
    } else {
        None
    };
    let games = if matches!(target, DetectionTarget::Games | DetectionTarget::All) {
        scan_games(&libraries(&root, &mut issues), &mut issues)
    } else {
        Vec::new()
    };
    DetectionResult {
        steam,
        games,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_steam_keyvalues_and_quoted_braces() {
        let parsed = parse("\"AppState\" { // comment\n \"name\" \"A \\\"Game\\\"\" \"installdir\" \"{game}\" \"path\" \"D:\\\\Games\" }").unwrap();
        let Some(Value::Object(app)) = field(&parsed, "appstate") else {
            panic!("missing object")
        };
        assert_eq!(text(app, "name"), Some("A \"Game\""));
        assert_eq!(text(app, "installdir"), Some("{game}"));
        assert_eq!(text(app, "path"), Some(r"D:\Games"));
        assert!(parse("\"AppState\" { \"name\" \"unterminated").is_err());
        assert!(parse("\"AppState\" { \"name\" \"test\"").is_err());
    }

    #[test]
    fn finds_installed_games_and_all_executable_candidates_across_libraries() {
        let temp = tempfile::tempdir().unwrap();
        let steam = temp.path().join("steam");
        let second = temp.path().join("library");
        for dir in [&steam, &second] {
            fs::create_dir_all(dir.join("steamapps").join("common")).unwrap();
        }
        let library_text = format!(
            "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" \"{}\" }}",
            steam.to_string_lossy().replace('\\', "\\\\"),
            second.to_string_lossy().replace('\\', "\\\\")
        );
        fs::write(
            steam.join("steamapps").join("libraryfolders.vdf"),
            library_text,
        )
        .unwrap();
        let game = second.join("steamapps").join("common").join("Game");
        fs::create_dir_all(game.join("bin")).unwrap();
        fs::write(game.join("launcher.exe"), b"fixture").unwrap();
        fs::write(game.join("bin").join("game.EXE"), b"fixture").unwrap();
        fs::write(second.join("steamapps").join("appmanifest_42.acf"), "\"AppState\" { \"appid\" \"42\" \"name\" \"Game\" \"StateFlags\" \"4\" \"installdir\" \"Game\" }").unwrap();
        fs::write(second.join("steamapps").join("appmanifest_43.acf"), "\"AppState\" { \"appid\" \"43\" \"name\" \"Not installed\" \"StateFlags\" \"2\" \"installdir\" \"Missing\" }").unwrap();
        fs::write(second.join("steamapps").join("appmanifest_44.acf"), "\"AppState\" { \"appid\" \"44\" \"name\" \"Invalid\" \"StateFlags\" \"4\" \"installdir\" \"..\\\\escape\" }").unwrap();
        let mut issues = Vec::new();
        let found = libraries(&steam, &mut issues);
        assert_eq!(found.len(), 2);
        let games = scan_games(&found, &mut issues);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_id, "42");
        assert_eq!(games[0].candidates.len(), 2);
        assert!(
            games[0]
                .candidates
                .iter()
                .any(|path| path.ends_with("game.EXE"))
        );
        assert!(
            games[0]
                .candidates
                .iter()
                .any(|path| path.ends_with("launcher.exe"))
        );
        assert!(issues.iter().any(|issue| issue.kind == "manifest"));
    }

    #[test]
    fn reports_missing_library_metadata_and_skips_linked_folders() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        let outside = temp.path().join("outside");
        fs::create_dir(&game).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("not-a-game.exe"), b"fixture").unwrap();
        let link = game.join("linked");
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:GO_TEST_LINK -Target $env:GO_TEST_TARGET -ErrorAction Stop | Out-Null"])
            .env("GO_TEST_LINK", &link)
            .env("GO_TEST_TARGET", &outside)
            .output()
            .unwrap();
        assert!(output.status.success());
        let mut issues = Vec::new();
        assert_eq!(
            libraries(temp.path(), &mut issues),
            vec![temp.path().to_owned()]
        );
        assert!(issues.iter().any(|issue| issue.kind == "library_metadata"));
        assert!(executable_candidates(&game, &mut issues).is_empty());
        assert!(issues.iter().any(|issue| issue.kind == "link_skipped"));
        fs::remove_dir(link).unwrap();
    }

    #[test]
    #[ignore = "read-only smoke test of the current user's Steam installation"]
    fn detects_current_steam_installation() {
        let found = detect(DetectionTarget::All);
        println!(
            "Steam: {}; installed manifests: {}; issues: {}",
            found.steam.is_some(),
            found.games.len(),
            found.issues.len()
        );
        assert!(found.steam.is_some(), "Steam was not detected");
    }
}
