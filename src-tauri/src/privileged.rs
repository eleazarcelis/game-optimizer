use crate::{
    defender,
    defender_coordination::{self, Coordinator, Reservation},
    gpu::{RawValue, raw, reg_type},
    machine_resource,
    resource_identity::{Identity, Target},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use uuid::Uuid;
use windows_sys::Win32::{
    Foundation::{CloseHandle, WAIT_OBJECT_0},
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    },
    System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
    UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
};
use winreg::{
    RegKey, RegValue,
    enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, KEY_WRITE},
};

const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Job {
    #[serde(rename = "cpu")]
    Cpu {
        name: String,
        owner: Uuid,
        expected: Option<RawValue>,
        desired: Option<RawValue>,
    },
    #[serde(rename = "defender")]
    Defender {
        resource: String,
        target: String,
        owner: Uuid,
        identity: Identity,
        expected: bool,
        desired: bool,
    },
    #[serde(rename = "defender-coordinated")]
    DefenderCoordinated {
        resource: String,
        path: String,
        target: String,
        owner: Uuid,
        identity: Identity,
        expected: bool,
        desired: bool,
        restore: bool,
    },
    #[serde(rename = "defender-query")]
    DefenderQuery { resource: String, target: String },
}

fn validate_name(name: &str) -> Result<(), String> {
    log::info!("Validating CPU image name");
    if name.is_empty()
        || name.len() > 255
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || !name.to_ascii_lowercase().ends_with(".exe")
        || name.ends_with(' ')
        || name.ends_with('.')
    {
        return Err("Invalid CPU image name".into());
    }
    Ok(())
}

fn key_path(name: &str) -> Result<String, String> {
    validate_name(name)?;
    Ok(format!(r"{IFEO}\{name}\PerfOptions"))
}

pub fn cpu_name(path: &str) -> Result<String, String> {
    log::info!("Resolving CPU image name");
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("Executable path must be absolute".into());
    }
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("Invalid executable name")?;
    validate_name(name)?;
    Ok(name.into())
}

pub fn read_cpu(name: &str) -> Result<Option<RawValue>, String> {
    log::info!("Reading CPU priority");
    let path = key_path(name)?;
    let root = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = match root.open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY) {
        Ok(key) => key,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot open CPU priority key: {e}")),
    };
    match key.get_raw_value("CpuPriorityClass") {
        Ok(value) => Ok(Some(raw(value))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read CPU priority: {e}")),
    }
}

fn write_cpu(name: &str, value: &Option<RawValue>) -> Result<(), String> {
    log::info!("Writing CPU priority");
    let path = key_path(name)?;
    let root = RegKey::predef(HKEY_LOCAL_MACHINE);
    match value {
        Some(value) => {
            let (key, _) = root
                .create_subkey_with_flags(path, KEY_READ | KEY_WRITE | KEY_WOW64_64KEY)
                .map_err(|e| format!("Cannot create CPU priority key: {e}"))?;
            key.set_raw_value(
                "CpuPriorityClass",
                &RegValue {
                    vtype: reg_type(value.kind)?,
                    bytes: Cow::Owned(value.bytes.clone()),
                },
            )
            .map_err(|e| format!("Cannot write CPU priority: {e}"))
        }
        None => {
            let key =
                match root.open_subkey_with_flags(path, KEY_READ | KEY_WRITE | KEY_WOW64_64KEY) {
                    Ok(key) => key,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(e) => return Err(format!("Cannot open CPU priority key: {e}")),
                };
            match key.delete_value("CpuPriorityClass") {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(format!("Cannot remove CPU priority: {e}")),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum DefenderTransition {
    Unchanged,
    Write,
    Release,
    WriteAndRelease,
}

fn defender_transition(
    restore: bool,
    folder: bool,
    owned: bool,
    current: bool,
    expected: bool,
    desired: bool,
) -> Result<DefenderTransition, String> {
    if restore && current == desired {
        return Ok(if folder && owned {
            DefenderTransition::Release
        } else {
            DefenderTransition::Unchanged
        });
    }
    if !owned {
        return Err("Defender reservation is missing".into());
    }
    if current != expected {
        return Err("Defender exclusion changed externally".into());
    }
    Ok(match (current != desired, restore && folder) {
        (false, false) => DefenderTransition::Unchanged,
        (true, false) => DefenderTransition::Write,
        (false, true) => DefenderTransition::Release,
        (true, true) => DefenderTransition::WriteAndRelease,
    })
}

fn execute(job: Job) -> Result<i32, String> {
    log::info!("Executing restricted administrative action");
    match job {
        Job::Cpu {
            name,
            owner,
            expected,
            desired,
        } => {
            validate_name(&name)?;
            let (reservation, lock_path) = machine_resource::paths("cpu", &name)?;
            let _guard = machine_resource::lock(&lock_path)?;
            if machine_resource::owner(&reservation)?.as_deref() != Some(owner.to_string().as_str())
            {
                return Err("CPU resource reservation does not match".into());
            }
            if read_cpu(&name)? != expected {
                return Err("CPU priority changed externally".into());
            }
            write_cpu(&name, &desired)?;
            if read_cpu(&name)? != desired {
                return Err("CPU priority verification failed".into());
            }
            Ok(0)
        }
        Job::Defender {
            resource,
            target,
            owner,
            identity,
            expected,
            desired,
        } => {
            defender::validate_target(&resource, &target)?;
            let held_target = Target::open(&target, resource == "defender-path")?;
            if held_target.identity() != &identity {
                return Err("Defender resource identity changed".into());
            }
            let (reservation, lock_path) =
                machine_resource::defender_identity_paths(&resource, &identity)?;
            let _guard = machine_resource::lock(&lock_path)?;
            if machine_resource::owner(&reservation)?.as_deref() != Some(owner.to_string().as_str())
            {
                return Err("Defender resource reservation does not match".into());
            }
            if defender::read(&resource, &target)? != expected {
                return Err("Defender exclusion changed externally".into());
            }
            held_target.ensure_path(&target, resource == "defender-path")?;
            defender::write(&resource, &target, desired)?;
            held_target.ensure_path(&target, resource == "defender-path")?;
            if defender::read(&resource, &target)? != desired {
                return Err("Defender exclusion verification failed".into());
            }
            Ok(0)
        }
        Job::DefenderCoordinated {
            resource,
            path,
            target,
            owner,
            identity,
            expected,
            desired,
            restore,
        } => {
            defender::validate_action_target(&resource, &path, &target)?;
            if !restore && !desired {
                return Err("Invalid Defender apply operation".into());
            }
            let held_target = Target::open(&target, resource == "defender-path")?;
            if held_target.identity() != &identity {
                return Err("Defender resource identity changed".into());
            }
            let (reservation, _held) = Reservation::open(&path, owner)?;
            let coordinator = Coordinator::open()?;
            if !restore && resource == "defender-path" {
                defender_coordination::ensure_no_legacy_reservations()?;
                coordinator.claim(&reservation)?;
            }
            let owned = coordinator.owned(&reservation)?;
            let current = defender::read(&resource, &target)?;
            let transition = defender_transition(
                restore,
                resource == "defender-path",
                owned,
                current,
                expected,
                desired,
            )?;
            held_target.ensure_path(&target, resource == "defender-path")?;
            if matches!(
                transition,
                DefenderTransition::Write | DefenderTransition::WriteAndRelease
            ) {
                defender::write(&resource, &target, desired)?;
            }
            held_target.ensure_path(&target, resource == "defender-path")?;
            if defender::read(&resource, &target)? != desired {
                return Err("Defender exclusion verification failed".into());
            }
            if matches!(
                transition,
                DefenderTransition::Release | DefenderTransition::WriteAndRelease
            ) {
                coordinator.release(&reservation)?;
            }
            Ok(0)
        }
        Job::DefenderQuery { resource, target } => {
            defender::validate_target(&resource, &target)?;
            Ok(if defender::read(&resource, &target)? {
                11
            } else {
                10
            })
        }
    }
}

pub fn helper(encoded: &str) -> i32 {
    let result = STANDARD
        .decode(encoded)
        .map_err(|e| format!("Invalid request encoding: {e}"))
        .and_then(|bytes| {
            serde_json::from_slice::<Job>(&bytes).map_err(|e| format!("Invalid request: {e}"))
        })
        .and_then(execute);
    match result {
        Ok(code) => code,
        Err(e) => {
            log::error!("ADMIN_ACTION_FAILED: {e}");
            eprintln!("ADMIN_ACTION_FAILED: {e}");
            match e.as_str() {
                "Defender exclusion overlaps another active profile" => 12,
                "Defender reservation directory has unsafe permissions or ownership" => 13,
                "Defender resource identity changed" | "Defender reservation identity changed" => {
                    14
                }
                "Defender reservation is missing" => 15,
                "Defender exclusion changed externally" => 16,
                "Restore previous Defender transactions before applying a new exclusion" => 17,
                _ => 1,
            }
        }
    }
}

pub fn launch(job: &Job) -> Result<(), String> {
    let code = launch_code(job)?;
    match code {
        0 => Ok(()),
        12 => Err("Defender exclusion overlaps another active profile".into()),
        13 => Err("Defender reservation directory has unsafe permissions or ownership".into()),
        14 => Err("Defender resource or reservation identity changed".into()),
        15 => Err("Defender reservation is missing".into()),
        16 => Err("Defender exclusion changed externally".into()),
        17 => Err("Restore previous Defender transactions before applying a new exclusion".into()),
        _ => Err(format!(
            "Administrator helper failed (exit code {code}); check permissions and external changes"
        )),
    }
}

pub fn query_defender(resource: &str, target: &str) -> Result<bool, String> {
    match launch_code(&Job::DefenderQuery {
        resource: resource.into(),
        target: target.into(),
    })? {
        10 => Ok(false),
        11 => Ok(true),
        code => Err(format!(
            "Administrator Defender query failed (exit code {code})"
        )),
    }
}

fn launch_code(job: &Job) -> Result<u32, String> {
    let payload = STANDARD.encode(serde_json::to_vec(job).map_err(|e| e.to_string())?);
    if payload.len() > 16000 {
        return Err("Administrative request is too large".into());
    }
    if unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() } != 0 {
        log::info!("Executing administrative action in elevated application");
        return Ok(helper(&payload) as u32);
    }
    log::info!("Requesting administrator approval");
    #[cfg(test)]
    let executable = match std::env::var_os("GAME_OPTIMIZER_HELPER_EXE") {
        Some(path) => std::path::PathBuf::from(path),
        None => std::env::current_exe().map_err(|e| format!("Cannot locate application: {e}"))?,
    };
    #[cfg(not(test))]
    let executable =
        std::env::current_exe().map_err(|e| format!("Cannot locate application: {e}"))?;
    std::thread::spawn(move || launch_inner(&executable, &payload))
        .join()
        .map_err(|_| "Administrator helper launcher panicked".to_string())?
}

fn launch_inner(executable: &Path, payload: &str) -> Result<u32, String> {
    let initialized = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if initialized < 0 {
        return Err(format!(
            "Cannot initialize administrator launcher: HRESULT {initialized:#x}"
        ));
    }
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let _com = ComGuard;
    let wide = |s: &OsStr| {
        s.encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let verb = wide(OsStr::new("runas"));
    let file = wide(executable.as_os_str());
    let args = wide(OsStr::new(&format!("--admin-helper {payload}")));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: args.as_ptr(),
        nShow: 0,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(format!(
            "Administrator approval failed or was cancelled: {}",
            std::io::Error::last_os_error()
        ));
    }
    if info.hProcess.is_null() {
        return Err("Administrator helper did not return a process handle".into());
    }
    let wait = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    let mut exit_code = 0;
    let queried = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe {
        CloseHandle(info.hProcess);
    }
    if wait != WAIT_OBJECT_0 || queried == 0 {
        return Err(format!(
            "Cannot wait for administrator helper: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an elevated test process; does not change Windows settings"]
    fn elevated_job_runs_without_another_helper() {
        assert_ne!(unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() }, 0);
        assert_eq!(
            launch_code(&Job::DefenderQuery {
                resource: "defender-process".into(),
                target: "invalid".into(),
            })
            .unwrap(),
            1
        );
    }

    #[test]
    fn rejects_registry_path_injection() {
        assert!(cpu_name(r"C:\Games\game.EXE").is_ok());
        assert!(validate_name(r"game.exe\PerfOptions").is_err());
        assert!(validate_name("game.exe:bad").is_err());
        assert!(validate_name("game.exe\0").is_err());
    }

    #[test]
    fn coordinated_restore_requires_ownership_for_changes_and_releases_last() {
        use DefenderTransition::*;
        assert_eq!(
            defender_transition(false, true, true, false, false, true).unwrap(),
            Write
        );
        assert_eq!(
            defender_transition(false, false, true, true, true, true).unwrap(),
            Unchanged
        );
        assert_eq!(
            defender_transition(true, false, true, true, true, false).unwrap(),
            Write
        );
        assert_eq!(
            defender_transition(true, true, true, true, true, false).unwrap(),
            WriteAndRelease
        );
        assert_eq!(
            defender_transition(true, true, true, false, true, false).unwrap(),
            Release
        );
        assert_eq!(
            defender_transition(true, true, false, false, true, false).unwrap(),
            Unchanged
        );
        assert!(defender_transition(true, true, false, true, true, false).is_err());
        assert!(defender_transition(false, true, true, true, false, true).is_err());
    }

    #[test]
    #[ignore = "requires UAC to inspect the dummy Defender exclusions"]
    fn query_dummy_defender_with_admin() {
        let exe = std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let folder = Path::new(&exe).parent().unwrap().to_str().unwrap();
        let elevated_folder = query_defender("defender-path", folder).unwrap();
        let elevated_process = query_defender("defender-process", &exe).unwrap();
        eprintln!(
            "Dummy folder excluded: {elevated_folder}; dummy process excluded: {elevated_process}"
        );
    }
}
