use crate::{AppError, error, executable};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    },
    UI::Shell::{SHELLEXECUTEINFOW, ShellExecuteExW},
};

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

pub fn launch(path: &str) -> Result<(), AppError> {
    log::info!("Launching executable");
    let path = executable(path)?;
    let directory = path
        .parent()
        .ok_or_else(|| error("LAUNCH_FAILED", "Executable has no parent directory"))?;
    crate::desktop::launch(&path, directory).map_err(|e| error("LAUNCH_FAILED", e))
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
    fn launch_and_folder_reject_invalid_paths() {
        assert_eq!(
            launch("relative.exe").unwrap_err().code,
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
            launch(invalid.to_str().unwrap()).unwrap_err().code,
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
        launch(path.to_str().unwrap()).unwrap();
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
