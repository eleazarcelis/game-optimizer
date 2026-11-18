use crate::{AppError, cpu::CpuPriority, error, executable};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows::{Management::Deployment::PackageManager, core::HSTRING};
use windows_sys::Win32::{
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    },
    UI::Shell::{SHELLEXECUTEINFOW, ShellExecuteExW},
};

fn package_directory(path: &Path) -> Option<&Path> {
    path.ancestors().find(|ancestor| {
        ancestor
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name.eq_ignore_ascii_case("WindowsApps"))
    })
}

fn select_package_entry(executable: &str, ids: &[String]) -> Result<usize, &'static str> {
    let matches: Vec<_> = ids
        .iter()
        .enumerate()
        .filter(|(_, id)| {
            id.rsplit_once('!')
                .is_some_and(|(_, name)| name.eq_ignore_ascii_case(executable))
        })
        .map(|(index, _)| index)
        .collect();
    match (matches.as_slice(), ids.len()) {
        ([index], _) => Ok(*index),
        ([], 1) => Ok(0),
        _ => Err("Cannot identify a unique packaged application for this executable"),
    }
}

fn launch_packaged(path: &Path, directory: &Path) -> Result<(), String> {
    crate::desktop::check_user()?;
    let result = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            windows_sys::Win32::System::Com::COINIT_MULTITHREADED as u32,
        )
    };
    if result < 0 {
        return Err(format!("COM initialization failed: {result:#x}"));
    }
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _guard = ComGuard;
    let package_name = directory.file_name().ok_or("Invalid package directory")?;
    let package = PackageManager::new()
        .and_then(|manager| {
            manager.FindPackageByUserSecurityIdPackageFullName(
                &HSTRING::new(),
                &HSTRING::from(package_name),
            )
        })
        .map_err(|e| format!("Cannot find the installed package: {e}"))?;
    let installed = package
        .InstalledLocation()
        .and_then(|folder| folder.Path())
        .map_err(|e| format!("Cannot identify the package installation directory: {e}"))?;
    if !directory
        .as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&installed.to_string_lossy())
    {
        return Err("The executable is not in the registered package directory".into());
    }
    let entries = package
        .GetAppListEntriesAsync()
        .and_then(|operation| operation.join())
        .map_err(|e| format!("Cannot enumerate packaged applications: {e}"))?;
    let mut apps = Vec::new();
    let mut ids = Vec::new();
    for entry in &entries {
        ids.push(
            entry
                .AppUserModelId()
                .map_err(|e| format!("Cannot identify packaged application: {e}"))?
                .to_string_lossy(),
        );
        apps.push(entry);
    }
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or("Invalid executable name")?;
    let index = select_package_entry(stem, &ids)?;
    let launched = apps[index]
        .LaunchAsync()
        .and_then(|operation| operation.join())
        .map_err(|e| format!("Cannot activate packaged application: {e}"))?;
    if !launched {
        return Err("Windows could not activate the packaged application".into());
    }
    Ok(())
}

fn open(path: &Path, working_directory: Option<&Path>, code: &'static str) -> Result<(), AppError> {
    log::info!("Opening a Windows shell target");
    let result = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if result < 0 {
        return Err(error(
            code,
            format!("COM initialization failed: {result:#x}"),
        ));
    }
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _guard = ComGuard;
    let wide = |s: &OsStr| {
        s.encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let file = wide(path.as_os_str());
    let directory = working_directory.map(|p| wide(p.as_os_str()));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        lpFile: file.as_ptr(),
        lpDirectory: directory.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
        nShow: 1,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(error(code, std::io::Error::last_os_error()));
    }
    Ok(())
}

pub fn launch(path: &str, priority: Option<CpuPriority>) -> Result<Option<&'static str>, AppError> {
    log::info!("Launching executable");
    let path = executable(path)?;
    let directory = path
        .parent()
        .ok_or_else(|| error("LAUNCH_FAILED", "Executable has no parent directory"))?;
    if let Some(package_dir) = package_directory(&path) {
        launch_packaged(&path, package_dir).map_err(|e| error("LAUNCH_FAILED", e))?;
        return Ok(priority.map(|_| "packagedPriorityUnavailable"));
    }
    crate::desktop::launch(&path, directory, priority)
        .map(|failed| failed.then_some("jobUnavailable"))
        .map_err(|e| error("LAUNCH_FAILED", e))
}

pub fn open_folder(path: &str) -> Result<(), AppError> {
    log::info!("Opening executable folder");
    let path = Path::new(path);
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return Err(error("INVALID_EXECUTABLE", "Select an absolute .exe path"));
    }
    let folder = path
        .parent()
        .filter(|parent| parent.is_dir())
        .ok_or_else(|| error("FOLDER_OPEN", "Executable folder does not exist"))?;
    open(folder, None, "FOLDER_OPEN")
}

pub fn open_update_download(tag: &str) -> Result<(), AppError> {
    if tag.len() > 201
        || !tag.starts_with('v')
        || !tag.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
        || !tag
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
    {
        return Err(error("UPDATE_DOWNLOAD", "Invalid release tag"));
    }
    let url = format!(
        "https://github.com/eleazarcelis/game-optimizer/releases/download/{tag}/Game-Optimizer-{tag}-windows-x64.exe"
    );
    open(Path::new(&url), None, "UPDATE_DOWNLOAD")
}

pub fn open_issue_report() -> Result<(), AppError> {
    open(
        Path::new("https://github.com/eleazarcelis/game-optimizer/issues/new"),
        None,
        "ISSUE_OPEN",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_app_selection_requires_an_unambiguous_match() {
        let ids = vec!["family!Spotify".into(), "family!SpotifyCli".into()];
        assert_eq!(select_package_entry("spotify", &ids), Ok(0));
        assert!(select_package_entry("other", &ids).is_err());
        assert_eq!(select_package_entry("other", &ids[..1]), Ok(0));
        assert!(select_package_entry("other", &[]).is_err());
        assert!(
            select_package_entry(
                "spotify",
                &["family!Spotify".into(), "other!spotify".into()]
            )
            .is_err()
        );
    }

    #[test]
    fn finds_package_directory_for_nested_executables() {
        assert_eq!(
            package_directory(Path::new(
                r"C:\Program Files\WindowsApps\package\bin\app.exe"
            )),
            Some(Path::new(r"C:\Program Files\WindowsApps\package"))
        );
        assert!(package_directory(Path::new(r"C:\Games\app.exe")).is_none());
    }

    #[test]
    fn launch_and_folder_reject_invalid_paths() {
        assert_eq!(
            launch("relative.exe", None).unwrap_err().code,
            "INVALID_EXECUTABLE"
        );
        assert_eq!(
            open_folder("relative.exe").unwrap_err().code,
            "INVALID_EXECUTABLE"
        );
        let dir = tempfile::tempdir().unwrap();
        let invalid = dir.path().join("not-executable.exe");
        std::fs::write(&invalid, b"not a program").unwrap();
        assert_eq!(
            launch(invalid.to_str().unwrap(), None).unwrap_err().code,
            "LAUNCH_FAILED"
        );
        let missing = dir.path().join("missing").join("game.exe");
        assert_eq!(
            open_folder(missing.to_str().unwrap()).unwrap_err().code,
            "FOLDER_OPEN"
        );
    }

    #[test]
    fn update_download_rejects_unsafe_tags() {
        for tag in ["", "v", "v1/../../other", "v1?bad", "v1#bad", "x1.2.3"] {
            assert_eq!(
                open_update_download(tag).unwrap_err().code,
                "UPDATE_DOWNLOAD"
            );
        }
    }

    #[test]
    #[ignore = "compiles and launches a disposable executable"]
    fn launch_disposable_executable() {
        let dir = tempfile::Builder::new()
            .prefix("game optimizer launch ")
            .tempdir()
            .unwrap();
        let path = dir.path().join("test game.exe");
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("launch_dummy.rs");
        assert!(
            std::process::Command::new("rustc")
                .arg(source)
                .arg("-o")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        launch(path.to_str().unwrap(), None).unwrap();
        let directory = path.parent().unwrap();
        let marker = directory.join("launch-marker.txt");
        for _ in 0..100 {
            if marker.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert_eq!(
            std::fs::read_to_string(marker).unwrap(),
            format!("{}\n0", directory.to_string_lossy())
        );
    }
}
