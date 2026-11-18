use base64::{Engine, engine::general_purpose::STANDARD};
use fs2::FileExt;
use serde::Serialize;
use serde_json::Value;
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};
use tauri::Manager;

#[cfg(windows)]
mod cpu;
#[cfg(windows)]
mod defender;
#[cfg(windows)]
mod defender_coordination;
#[cfg(windows)]
mod desktop;
#[cfg(windows)]
mod executable_name;
#[cfg(windows)]
mod global;
#[cfg(windows)]
pub mod gpu;
#[cfg(windows)]
mod gpu_service;
#[cfg(windows)]
mod launcher;
#[cfg(windows)]
mod machine_resource;
#[cfg(windows)]
mod power;
#[cfg(windows)]
mod privileged;
#[cfg(windows)]
mod resource_identity;
#[cfg(windows)]
mod steam_detection;
pub mod transaction;

#[derive(Debug, Serialize)]
struct AppError {
    code: &'static str,
    message: String,
}

fn error(code: &'static str, detail: impl std::fmt::Display) -> AppError {
    log::error!("{code}: {detail}");
    AppError {
        code,
        message: detail.to_string(),
    }
}

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    log::info!("Resolving application data directory");
    let path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| error("DATA_PATH", e))?;
    std::fs::create_dir_all(&path).map_err(|e| error("DATA_CREATE", e))?;
    Ok(path)
}

struct InstanceAccess {
    _lock: Option<std::fs::File>,
    failure: Option<String>,
}

fn instance_lock(path: &Path) -> Result<std::fs::File, AppError> {
    log::info!("Locking application data");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path.join("app.lock"))
        .map_err(|e| error("INSTANCE_LOCK", e))?;
    file.try_lock_exclusive()
        .map_err(|e| error("INSTANCE_BUSY", e))?;
    Ok(file)
}

fn require_instance(app: &tauri::AppHandle) -> Result<(), AppError> {
    log::info!("Checking application data lock");
    let access = app.state::<InstanceAccess>();
    match &access.failure {
        Some(message) => Err(error("INSTANCE_UNAVAILABLE", message)),
        None => Ok(()),
    }
}

#[tauri::command]
async fn load_state(app: tauri::AppHandle) -> Result<Option<Value>, AppError> {
    log::info!("Loading application state");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        let path = data_dir(&app)?.join("state.json");
        read_state_file(&path)
    })
    .await
    .map_err(|e| error("DATA_TASK", e))?
}

fn read_state_file(path: &Path) -> Result<Option<Value>, AppError> {
    log::info!("Reading application state file");
    match std::fs::read(path) {
        Ok(bytes) => {
            let state: Value =
                serde_json::from_slice(&bytes).map_err(|e| error("DATA_INVALID", e))?;
            if state.is_null() {
                return Err(error("DATA_INVALID", "Application state is null"));
            }
            Ok(Some(state))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error("DATA_READ", e)),
    }
}

#[tauri::command]
async fn save_state(app: tauri::AppHandle, state: Value) -> Result<(), AppError> {
    log::info!("Saving application state");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        let dir = data_dir(&app)?;
        let bytes = serde_json::to_vec_pretty(&state).map_err(|e| error("DATA_ENCODE", e))?;
        if bytes.len() > 5_000_000 {
            return Err(error("DATA_TOO_LARGE", "Application state exceeds 5 MB"));
        }
        let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| error("DATA_WRITE", e))?;
        use std::io::Write;
        temp.write_all(&bytes).map_err(|e| error("DATA_WRITE", e))?;
        temp.as_file()
            .sync_all()
            .map_err(|e| error("DATA_WRITE", e))?;
        temp.persist(dir.join("state.json"))
            .map_err(|e| error("DATA_WRITE", e))?;
        Ok(())
    })
    .await
    .map_err(|e| error("DATA_TASK", e))?
}

fn executable(path: &str) -> Result<PathBuf, AppError> {
    log::info!("Validating executable");
    let path = PathBuf::from(path);
    if !path.is_absolute()
        || !path.is_file()
        || !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
    {
        return Err(error("INVALID_EXECUTABLE", "Select an existing .exe file"));
    }
    Ok(path)
}

fn read_icon(path: String) -> Result<String, AppError> {
    log::info!("Reading executable icon");
    let path = executable(&path)?;
    let icon = file_icon_provider::get_file_icon(&path, 64).map_err(|e| error("ICON_READ", e))?;
    let bitmap = image::RgbaImage::from_raw(icon.width, icon.height, icon.pixels)
        .ok_or_else(|| error("ICON_FORMAT", "Invalid icon data"))?;
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(bitmap)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| error("ICON_ENCODE", e))?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(output.into_inner())
    ))
}

#[tauri::command]
async fn inspect_executable(path: String) -> Result<String, AppError> {
    log::info!("Inspecting executable");
    tauri::async_runtime::spawn_blocking(move || read_icon(path))
        .await
        .map_err(|e| error("ICON_TASK", e))?
}

#[tauri::command]
async fn executable_name(path: String) -> Result<Option<String>, AppError> {
    log::info!("Inspecting executable name");
    tauri::async_runtime::spawn_blocking(move || executable_name::read(&path))
        .await
        .map_err(|e| error("NAME_TASK", e))?
}

#[tauri::command]
async fn detect_steam(
    target: steam_detection::DetectionTarget,
) -> Result<steam_detection::DetectionResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || steam_detection::detect(target))
        .await
        .map_err(|e| error("DETECTION_TASK", e))
}

#[tauri::command]
async fn validate_detected_executables(paths: Vec<String>) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        for path in paths {
            executable(&path)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| error("DETECTION_TASK", e))?
}

#[tauri::command]
async fn launch_executable(
    app: tauri::AppHandle,
    path: String,
) -> Result<Option<&'static str>, AppError> {
    log::info!("Requesting executable launch");
    tauri::async_runtime::spawn_blocking(move || {
        executable(&path)?;
        let priority = require_instance(&app).and_then(|()| {
            gpu_service::launch_cpu_priority(&data_dir(&app)?.join("transactions"), &path)
        });
        let (priority, mut warning) = match priority {
            Ok(priority) => (priority, None),
            Err(e) => {
                log::warn!(
                    "Cannot determine CPU priority for launch: {}: {}",
                    e.code,
                    e.message
                );
                (None, Some("priorityUnavailable"))
            }
        };
        if let Some(launch_warning) = launcher::launch(&path, priority)? {
            warning = Some(launch_warning);
        }
        Ok(warning)
    })
    .await
    .map_err(|e| error("LAUNCH_TASK", e))?
}

#[tauri::command]
async fn open_executable_folder(path: String) -> Result<(), AppError> {
    log::info!("Requesting executable folder");
    tauri::async_runtime::spawn_blocking(move || launcher::open_folder(&path))
        .await
        .map_err(|e| error("FOLDER_TASK", e))?
}

#[tauri::command]
async fn open_update_download(tag: String) -> Result<(), AppError> {
    log::info!("Opening update download");
    tauri::async_runtime::spawn_blocking(move || launcher::open_update_download(&tag))
        .await
        .map_err(|e| error("UPDATE_TASK", e))?
}

#[tauri::command]
async fn open_issue_report() -> Result<(), AppError> {
    log::info!("Opening GitHub issue page");
    tauri::async_runtime::spawn_blocking(launcher::open_issue_report)
        .await
        .map_err(|e| error("ISSUE_TASK", e))?
}

#[tauri::command]
async fn gpu_status(
    app: tauri::AppHandle,
    path: String,
) -> Result<gpu_service::GpuStatus, AppError> {
    log::info!("Requesting GPU status");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        gpu_service::status(&data_dir(&app)?.join("transactions"), &path)
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

#[tauri::command]
async fn apply_profile(
    app: tauri::AppHandle,
    path: String,
    profile: gpu_service::ProfileActions,
) -> Result<Vec<transaction::ApplyWarning>, AppError> {
    log::info!("Requesting profile application");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        gpu_service::apply(
            &data_dir(&app)?.join("transactions"),
            &path,
            &profile,
            false,
        )
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

#[tauri::command]
async fn reset_profile(
    app: tauri::AppHandle,
    path: String,
    profile: gpu_service::ProfileActions,
) -> Result<(), AppError> {
    log::info!("Requesting profile defaults");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        gpu_service::apply(&data_dir(&app)?.join("transactions"), &path, &profile, true).map(|_| ())
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

#[tauri::command]
async fn restore_profile(
    app: tauri::AppHandle,
    path: String,
) -> Result<Vec<transaction::SkippedStep>, AppError> {
    log::info!("Requesting profile restoration");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        gpu_service::restore(&data_dir(&app)?.join("transactions"), &path)
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

#[tauri::command]
async fn global_status(app: tauri::AppHandle) -> Result<global::Status, AppError> {
    log::info!("Requesting global transaction status");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        global::status(&data_dir(&app)?.join("transactions"))
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

#[tauri::command]
async fn apply_global(app: tauri::AppHandle, settings: global::Settings) -> Result<(), AppError> {
    log::info!("Requesting global settings application");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        global::apply(&data_dir(&app)?.join("transactions"), &settings)
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

#[tauri::command]
async fn restore_global(app: tauri::AppHandle) -> Result<Vec<transaction::SkippedStep>, AppError> {
    log::info!("Requesting global settings restoration");
    tauri::async_runtime::spawn_blocking(move || {
        require_instance(&app)?;
        global::restore(&data_dir(&app)?.join("transactions"))
    })
    .await
    .map_err(|e| error("ACTION_TASK", e))?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("game-optimizer".into()),
                    },
                )])
                .build(),
        )
        .setup(|app| {
            log::info!("Starting desktop application");
            let access = match data_dir(app.handle()).and_then(|path| instance_lock(&path)) {
                Ok(lock) => InstanceAccess {
                    _lock: Some(lock),
                    failure: None,
                },
                Err(e) => InstanceAccess {
                    _lock: None,
                    failure: Some(format!("{}: {}", e.code, e.message)),
                },
            };
            app.manage(access);
            #[cfg(windows)]
            match app.get_webview_window("main") {
                Some(window) => {
                    if let Err(e) = window.set_focus() {
                        log::warn!("Could not focus the main window at startup: {e}");
                    }
                }
                None => log::error!("Could not find the main window at startup"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_state,
            save_state,
            inspect_executable,
            executable_name,
            detect_steam,
            validate_detected_executables,
            launch_executable,
            open_executable_folder,
            open_update_download,
            open_issue_report,
            gpu_status,
            apply_profile,
            reset_profile,
            restore_profile,
            global_status,
            apply_global,
            restore_global
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(windows)]
pub fn run_admin_helper(encoded: &str) -> i32 {
    privileged::helper(encoded)
}

#[cfg(windows)]
pub fn check_desktop() -> Result<(), String> {
    desktop::check_user()
}

#[cfg(test)]
mod tests {
    use super::{executable, instance_lock, read_icon, read_state_file};

    #[test]
    fn prevents_multiple_instances_using_the_same_data() {
        let dir = tempfile::tempdir().unwrap();
        let lock = instance_lock(dir.path()).unwrap();
        assert_eq!(instance_lock(dir.path()).unwrap_err().code, "INSTANCE_BUSY");
        drop(lock);
        assert!(instance_lock(dir.path()).is_ok());
    }

    #[test]
    fn executable_requires_an_existing_absolute_exe_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.EXE");
        std::fs::write(&path, b"test").unwrap();
        assert!(executable(path.to_str().unwrap()).is_ok());
        assert!(executable("relative.exe").is_err());
        assert!(executable(dir.path().to_str().unwrap()).is_err());
        assert!(executable(dir.path().join("missing.exe").to_str().unwrap()).is_err());
        let text = dir.path().join("game.exe.txt");
        std::fs::write(&text, b"test").unwrap();
        assert!(executable(text.to_str().unwrap()).is_err());
    }

    #[test]
    fn reads_an_icon_from_a_windows_executable() {
        let exe = std::env::current_exe().unwrap();
        let icon = read_icon(exe.to_str().unwrap().to_string()).unwrap();
        assert!(icon.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn missing_state_is_distinct_from_corrupt_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        assert!(read_state_file(&path).unwrap().is_none());
        std::fs::write(&path, "null").unwrap();
        assert_eq!(read_state_file(&path).unwrap_err().code, "DATA_INVALID");
        std::fs::write(&path, "{").unwrap();
        assert_eq!(read_state_file(&path).unwrap_err().code, "DATA_INVALID");
        std::fs::write(&path, r#"{"version":1}"#).unwrap();
        assert_eq!(read_state_file(&path).unwrap().unwrap()["version"], 1);
    }
}
