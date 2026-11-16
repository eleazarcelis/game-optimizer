use crate::{
    machine_resource,
    privileged::{self, Job},
    resource_identity::{Identity, Target},
    transaction::{Step, StepExecutor},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    os::windows::{ffi::OsStringExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;
use windows_sys::Win32::{
    System::SystemInformation::GetSystemDirectoryW, UI::Shell::IsUserAnAdmin,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    kind: String,
    path: String,
    target: String,
    owner: Uuid,
    enabled: bool,
    #[serde(default)]
    coordinated: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    excluded: bool,
    #[serde(default)]
    identity: Option<Identity>,
}

pub fn validate_target(kind: &str, target: &str) -> Result<(), String> {
    log::info!("Validating Defender exclusion target");
    let path = Path::new(target);
    if target.contains('\0') || target.len() > 8192 || !path.is_absolute() {
        return Err("Invalid Defender exclusion path".into());
    }
    match kind {
        "defender-process"
            if path
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("exe")) =>
        {
            Ok(())
        }
        "defender-path" if path.parent().is_some() => Ok(()),
        _ => Err("Invalid Defender exclusion target".into()),
    }
}

fn action(step: &Step) -> Result<Action, String> {
    let action: Action = serde_json::from_value(step.action.clone()).map_err(|e| e.to_string())?;
    validate_action_target(&action.kind, &action.path, &action.target)?;
    Ok(action)
}

pub fn validate_action_target(kind: &str, executable: &str, target: &str) -> Result<(), String> {
    validate_target(kind, target)?;
    let path = Path::new(executable);
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
    {
        return Err("Invalid executable path".into());
    }
    let expected = if kind == "defender-process" {
        path
    } else {
        path.parent().ok_or("Missing executable folder")?
    };
    if !expected.to_string_lossy().eq_ignore_ascii_case(target) {
        return Err("Defender target does not match executable".into());
    }
    Ok(())
}

pub fn validate(step: &Step, path: &str) -> Result<(), String> {
    let action = action(step)?;
    if step.id != action.kind || !action.path.eq_ignore_ascii_case(path) {
        return Err("Defender journal path mismatch".into());
    }
    Ok(())
}

fn powershell(script: &str) -> Result<String, String> {
    log::info!("Running restricted Defender query");
    let mut buffer = vec![0u16; 32768];
    let len = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if len == 0 || len >= buffer.len() {
        return Err(format!(
            "Cannot locate Windows system directory: {}",
            std::io::Error::last_os_error()
        ));
    }
    buffer.truncate(len);
    let executable = PathBuf::from(std::ffi::OsString::from_wide(&buffer))
        .join(r"WindowsPowerShell\v1.0\powershell.exe");
    let encoded = STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let output = Command::new(executable)
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Cannot start Defender command: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Defender command failed (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("Invalid Defender output: {e}"))
}

fn script(kind: &str, target: &str, operation: &str) -> Result<String, String> {
    validate_target(kind, target)?;
    let property = match kind {
        "defender-path" => "ExclusionPath",
        "defender-process" => "ExclusionProcess",
        _ => return Err("Invalid Defender kind".into()),
    };
    let encoded = STANDARD.encode(target.as_bytes());
    let setup = format!(
        "$ErrorActionPreference='Stop'; $target=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{encoded}')); "
    );
    let body = match operation {
        "read" => format!(
            "$p=Get-MpPreference -ErrorAction Stop; $t=[Environment]::ExpandEnvironmentVariables($target).TrimEnd('\\'); $found=@($p.{property} | Where-Object {{ $_ -and [string]::Equals([Environment]::ExpandEnvironmentVariables($_).TrimEnd('\\'),$t,[StringComparison]::OrdinalIgnoreCase) }}).Count -gt 0; if($found){{'1'}}else{{'0'}}"
        ),
        "add" => format!("Add-MpPreference -{property} $target -ErrorAction Stop"),
        "remove" => format!("Remove-MpPreference -{property} $target -ErrorAction Stop"),
        _ => return Err("Invalid Defender operation".into()),
    };
    Ok(setup + &body)
}

pub fn read(kind: &str, target: &str) -> Result<bool, String> {
    log::info!("Reading Defender exclusion");
    if unsafe { IsUserAnAdmin() } == 0 {
        return Err("Defender exclusion state requires an elevated administrator token".into());
    }
    match powershell(&script(kind, target, "read")?)?.trim() {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err("Unexpected Defender query result".into()),
    }
}

pub fn write(kind: &str, target: &str, desired: bool) -> Result<(), String> {
    log::info!("Changing Defender exclusion");
    powershell(&script(
        kind,
        target,
        if desired { "add" } else { "remove" },
    )?)?;
    Ok(())
}

trait Defender {
    fn identity(&mut self, kind: &str, target: &str) -> Result<Identity, String>;
    fn read(&mut self, kind: &str, target: &str) -> Result<bool, String>;
    fn write(
        &mut self,
        kind: &str,
        target: &str,
        owner: Uuid,
        identity: &Identity,
        expected: bool,
        desired: bool,
    ) -> Result<(), String>;
}
struct WindowsDefender;
impl Defender for WindowsDefender {
    fn identity(&mut self, kind: &str, target: &str) -> Result<Identity, String> {
        Ok(Target::open(target, kind == "defender-path")?
            .identity()
            .clone())
    }

    fn read(&mut self, kind: &str, target: &str) -> Result<bool, String> {
        privileged::query_defender(kind, target)
    }
    fn write(
        &mut self,
        kind: &str,
        target: &str,
        owner: Uuid,
        identity: &Identity,
        expected: bool,
        desired: bool,
    ) -> Result<(), String> {
        privileged::launch(&Job::Defender {
            resource: kind.into(),
            target: target.into(),
            owner,
            identity: identity.clone(),
            expected,
            desired,
        })
    }
}

pub struct DefenderExecutor;
impl DefenderExecutor {
    fn executor(step: &Step) -> Result<Executor<WindowsDefender>, String> {
        let action = action(step)?;
        let identity = match &step.before {
            Some(before) => serde_json::from_value::<Snapshot>(before.clone())
                .map_err(|e| e.to_string())?
                .identity
                .ok_or("Defender journal has no resource identity; manual recovery is required")?,
            None => Target::open(&action.target, action.kind == "defender-path")?
                .identity()
                .clone(),
        };
        let (reservation, lock) =
            machine_resource::defender_identity_paths(&action.kind, &identity)?;
        Ok(Executor {
            defender: WindowsDefender,
            reservation,
            lock,
        })
    }
}
impl StepExecutor for DefenderExecutor {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        let action = action(step)?;
        if action.coordinated {
            let target = Target::open(&action.target, action.kind == "defender-path")?;
            let excluded = privileged::query_defender(&action.kind, &action.target)?;
            target.ensure_path(&action.target, action.kind == "defender-path")?;
            return serde_json::to_value(Snapshot {
                excluded,
                identity: Some(target.identity().clone()),
            })
            .map_err(|e| e.to_string());
        }
        Self::executor(step)?.snapshot(step)
    }
    fn apply(&mut self, step: &Step) -> Result<(), String> {
        let action = action(step)?;
        if action.coordinated {
            let before: Snapshot =
                serde_json::from_value(step.before.clone().ok_or("Missing Defender snapshot")?)
                    .map_err(|e| e.to_string())?;
            return privileged::launch(&Job::DefenderCoordinated {
                resource: action.kind,
                path: action.path,
                target: action.target,
                owner: action.owner,
                identity: before
                    .identity
                    .ok_or("Defender journal has no resource identity")?,
                expected: before.excluded,
                desired: action.enabled,
                restore: false,
            });
        }
        Self::executor(step)?.apply(step)
    }
    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        let action = action(step)?;
        if action.coordinated {
            let before: Snapshot =
                serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
            return privileged::launch(&Job::DefenderCoordinated {
                resource: action.kind,
                path: action.path,
                target: action.target,
                owner: action.owner,
                identity: before
                    .identity
                    .ok_or("Defender journal has no resource identity")?,
                expected: action.enabled,
                desired: before.excluded,
                restore: true,
            });
        }
        Self::executor(step)?.restore(step, before)
    }
}

struct Executor<P> {
    defender: P,
    reservation: PathBuf,
    lock: PathBuf,
}
fn verify_identity<P: Defender>(
    defender: &mut P,
    action: &Action,
    snapshot: &Snapshot,
) -> Result<Identity, String> {
    let expected = snapshot
        .identity
        .as_ref()
        .ok_or("Defender journal has no resource identity; manual recovery is required")?;
    if &defender.identity(&action.kind, &action.target)? != expected {
        return Err("Defender resource identity changed; refusing to modify it".into());
    }
    Ok(expected.clone())
}
impl<P: Defender> StepExecutor for Executor<P> {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        log::info!("Capturing Defender exclusion");
        let action = action(step)?;
        let _guard = machine_resource::lock(&self.lock)?;
        let identity = self.defender.identity(&action.kind, &action.target)?;
        let excluded = self.defender.read(&action.kind, &action.target)?;
        if self.defender.identity(&action.kind, &action.target)? != identity {
            return Err("Defender resource identity changed during snapshot".into());
        }
        serde_json::to_value(Snapshot {
            excluded,
            identity: Some(identity),
        })
        .map_err(|e| e.to_string())
    }
    fn apply(&mut self, step: &Step) -> Result<(), String> {
        log::info!("Applying Defender exclusion");
        let action = action(step)?;
        let before: Snapshot =
            serde_json::from_value(step.before.clone().ok_or("Missing Defender snapshot")?)
                .map_err(|e| e.to_string())?;
        let identity = verify_identity(&mut self.defender, &action, &before)?;
        {
            let _guard = machine_resource::lock(&self.lock)?;
            machine_resource::claim(&self.reservation, &action.owner.to_string())?;
            if self.defender.read(&action.kind, &action.target)? != before.excluded {
                return Err("Defender exclusion changed since snapshot".into());
            }
            verify_identity(&mut self.defender, &action, &before)?;
        }
        if before.excluded != action.enabled {
            self.defender.write(
                &action.kind,
                &action.target,
                action.owner,
                &identity,
                before.excluded,
                action.enabled,
            )?;
        }
        let _guard = machine_resource::lock(&self.lock)?;
        if machine_resource::owner(&self.reservation)?.as_deref()
            != Some(action.owner.to_string().as_str())
            || self.defender.read(&action.kind, &action.target)? != action.enabled
        {
            return Err("Defender exclusion verification failed".into());
        }
        verify_identity(&mut self.defender, &action, &before)?;
        Ok(())
    }
    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        log::info!("Restoring Defender exclusion");
        let action = action(step)?;
        let before: Snapshot = serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
        let identity = verify_identity(&mut self.defender, &action, &before)?;
        let expected = action.owner.to_string();
        {
            let _guard = machine_resource::lock(&self.lock)?;
            let current = self.defender.read(&action.kind, &action.target)?;
            verify_identity(&mut self.defender, &action, &before)?;
            let owner = machine_resource::owner(&self.reservation)?;
            if current == before.excluded {
                if owner.as_deref() == Some(&expected) {
                    machine_resource::release(&self.reservation, &expected)?;
                }
                return Ok(());
            }
            if owner.as_deref() != Some(&expected) || current != action.enabled {
                return Err("Defender exclusion or reservation changed externally; refusing to overwrite it".into());
            }
        }
        self.defender.write(
            &action.kind,
            &action.target,
            action.owner,
            &identity,
            action.enabled,
            before.excluded,
        )?;
        let _guard = machine_resource::lock(&self.lock)?;
        if self.defender.read(&action.kind, &action.target)? != before.excluded {
            return Err("Defender exclusion verification failed after restoration".into());
        }
        verify_identity(&mut self.defender, &action, &before)?;
        machine_resource::release(&self.reservation, &expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{StepRequest, Transaction};
    use serde_json::json;
    struct Fake(bool);
    impl Defender for Fake {
        fn identity(&mut self, _: &str, _: &str) -> Result<Identity, String> {
            Ok(Identity {
                volume: 1,
                file_id: [1; 16],
            })
        }
        fn read(&mut self, _: &str, _: &str) -> Result<bool, String> {
            Ok(self.0)
        }
        fn write(
            &mut self,
            _: &str,
            _: &str,
            _: Uuid,
            _: &Identity,
            expected: bool,
            desired: bool,
        ) -> Result<(), String> {
            if self.0 != expected {
                return Err("conflict".into());
            }
            self.0 = desired;
            Ok(())
        }
    }

    #[test]
    fn restores_new_exclusions_but_preserves_preexisting_ones() {
        for original in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let request = StepRequest {
                id: "defender-process".into(),
                action: json!({"kind":"defender-process","path":r"C:\Games\demo.exe","target":r"C:\Games\demo.exe","owner":Uuid::new_v4(),"enabled":true}),
            };
            let mut tx = Transaction::create(&dir.path().join("tx.json"), &[request]).unwrap();
            let mut executor = Executor {
                defender: Fake(original),
                reservation: dir.path().join("resource"),
                lock: dir.path().join("resource.lock"),
            };
            tx.apply(&mut executor).unwrap();
            assert!(executor.defender.0);
            tx.restore(&mut executor).unwrap();
            assert_eq!(executor.defender.0, original);
            assert_eq!(
                machine_resource::owner(&executor.reservation).unwrap(),
                None
            );
        }
    }

    #[test]
    fn defaults_remove_an_existing_exclusion_and_restore_it() {
        let dir = tempfile::tempdir().unwrap();
        let request = StepRequest {
            id: "defender-process".into(),
            action: json!({"kind":"defender-process","path":r"C:\Games\demo.exe","target":r"C:\Games\demo.exe","owner":Uuid::new_v4(),"enabled":false}),
        };
        let mut tx = Transaction::create(&dir.path().join("tx.json"), &[request]).unwrap();
        let mut executor = Executor {
            defender: Fake(true),
            reservation: dir.path().join("resource"),
            lock: dir.path().join("resource.lock"),
        };
        tx.apply(&mut executor).unwrap();
        assert!(!executor.defender.0);
        tx.restore(&mut executor).unwrap();
        assert!(executor.defender.0);
    }

    #[test]
    fn failed_verification_keeps_restore_recoverable() {
        struct Unavailable;
        impl Defender for Unavailable {
            fn identity(&mut self, _: &str, _: &str) -> Result<Identity, String> {
                Ok(Identity {
                    volume: 1,
                    file_id: [1; 16],
                })
            }
            fn read(&mut self, _: &str, _: &str) -> Result<bool, String> {
                Err("Elevated query unavailable".into())
            }
            fn write(
                &mut self,
                _: &str,
                _: &str,
                _: Uuid,
                _: &Identity,
                _: bool,
                _: bool,
            ) -> Result<(), String> {
                panic!("Must not write without a verified state");
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let request = StepRequest {
            id: "defender-process".into(),
            action: json!({"kind":"defender-process","path":r"C:\Games\demo.exe","target":r"C:\Games\demo.exe","owner":Uuid::new_v4(),"enabled":true}),
        };
        let path = dir.path().join("tx.json");
        let mut tx = Transaction::create(&path, &[request]).unwrap();
        let reservation = dir.path().join("resource");
        let lock = dir.path().join("resource.lock");
        let mut executor = Executor {
            defender: Fake(false),
            reservation: reservation.clone(),
            lock: lock.clone(),
        };
        tx.apply(&mut executor).unwrap();
        let mut unavailable = Executor {
            defender: Unavailable,
            reservation,
            lock,
        };
        assert_eq!(
            tx.restore(&mut unavailable).unwrap_err().code,
            "RESTORE_FAILED"
        );
        drop(tx);
        let mut tx = Transaction::open(&path).unwrap();
        assert_eq!(
            tx.journal.steps[0].status,
            crate::transaction::StepStatus::Restoring
        );
        tx.restore(&mut executor).unwrap();
        assert!(!executor.defender.0);
    }

    #[test]
    fn changed_or_missing_identity_prevents_recovery_write() {
        struct Replaced;
        impl Defender for Replaced {
            fn identity(&mut self, _: &str, _: &str) -> Result<Identity, String> {
                Ok(Identity {
                    volume: 1,
                    file_id: [2; 16],
                })
            }
            fn read(&mut self, _: &str, _: &str) -> Result<bool, String> {
                panic!("Must not read after an identity mismatch");
            }
            fn write(
                &mut self,
                _: &str,
                _: &str,
                _: Uuid,
                _: &Identity,
                _: bool,
                _: bool,
            ) -> Result<(), String> {
                panic!("Must not write after an identity mismatch");
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let request = StepRequest {
            id: "defender-process".into(),
            action: json!({"kind":"defender-process","path":r"C:\Games\demo.exe","target":r"C:\Games\demo.exe","owner":Uuid::new_v4(),"enabled":true}),
        };
        let mut tx = Transaction::create(&dir.path().join("tx.json"), &[request]).unwrap();
        let reservation = dir.path().join("resource");
        let lock = dir.path().join("resource.lock");
        let mut original = Executor {
            defender: Fake(false),
            reservation: reservation.clone(),
            lock: lock.clone(),
        };
        tx.apply(&mut original).unwrap();
        let mut replaced = Executor {
            defender: Replaced,
            reservation,
            lock,
        };
        let error = tx.restore(&mut replaced).unwrap_err();
        assert_eq!(error.code, "RESTORE_FAILED");
        assert!(error.message.contains("identity changed"));
        let step = &tx.journal.steps[0];
        let old_snapshot = json!({"excluded": false});
        assert!(
            replaced
                .restore(step, &old_snapshot)
                .unwrap_err()
                .contains("no resource identity")
        );
        tx.restore(&mut original).unwrap();
    }

    #[test]
    #[ignore = "requires repeated UAC approvals and temporarily changes Defender exclusions for a disposable executable"]
    fn real_dummy_defender_applies_and_restores() {
        let exe = std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_DEFENDER_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_DEFENDER_TEST_ROOT"),
        );
        std::fs::create_dir_all(&root).unwrap();
        let folder = Path::new(&exe).parent().unwrap().to_str().unwrap();
        assert!(!privileged::query_defender("defender-path", folder).unwrap());
        assert!(!privileged::query_defender("defender-process", &exe).unwrap());
        let requests: Vec<_> = [("defender-path", folder), ("defender-process", exe.as_str())]
            .into_iter()
            .map(|(kind, target)| StepRequest {
                id: kind.into(),
                action: json!({"kind":kind,"path":exe,"target":target,"owner":Uuid::new_v4(),"enabled":true}),
            })
            .collect();
        let path = root.join("defender-journal.json");
        struct Guard(PathBuf);
        impl Drop for Guard {
            fn drop(&mut self) {
                if self.0.exists() {
                    if let Err(e) = Transaction::open(&self.0)
                        .and_then(|mut tx| tx.restore(&mut DefenderExecutor))
                    {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        let _guard = Guard(path.clone());
        let mut tx = Transaction::create(&path, &requests).unwrap();
        tx.apply(&mut DefenderExecutor).unwrap();
        assert!(privileged::query_defender("defender-path", folder).unwrap());
        assert!(privileged::query_defender("defender-process", &exe).unwrap());
        tx.restore(&mut DefenderExecutor).unwrap();
        assert!(!privileged::query_defender("defender-path", folder).unwrap());
        assert!(!privileged::query_defender("defender-process", &exe).unwrap());
    }

    #[test]
    #[ignore = "requires UAC to restore the disposable Defender test journal"]
    fn recover_dummy_defender_journal() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_DEFENDER_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_DEFENDER_TEST_ROOT"),
        );
        let mut tx = Transaction::open(&root.join("defender-journal.json")).unwrap();
        tx.restore(&mut DefenderExecutor).unwrap();
    }

    #[test]
    fn scripts_encode_paths_instead_of_interpolating_them() {
        let target = r"C:\Games\';throw 'injected\game.exe";
        let generated = script("defender-process", target, "read").unwrap();
        assert!(!generated.contains(target));
        assert!(validate_target("defender-path", r"C:\").is_err());
    }
}
