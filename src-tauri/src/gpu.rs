use crate::transaction::{Step, StepExecutor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{borrow::Cow, io, path::Path};
use winreg::{
    RegKey, RegValue,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, RegType},
    types::ToRegValue,
};

const KEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GpuAction {
    kind: String,
    path: String,
    preference: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawValue {
    pub(crate) kind: u32,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    value: Option<RawValue>,
}

fn action(step: &Step) -> Result<GpuAction, String> {
    log::info!("Validating GPU transaction action");
    let action: GpuAction =
        serde_json::from_value(step.action.clone()).map_err(|e| e.to_string())?;
    let path = Path::new(&action.path);
    if action.kind != "gpu"
        || !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        || !matches!(action.preference, 0..=2)
    {
        return Err("Invalid GPU action".into());
    }
    Ok(action)
}

pub fn validate(step: &Step, path: &str) -> Result<(), String> {
    let action = action(step)?;
    if !action.path.eq_ignore_ascii_case(path) {
        return Err("GPU journal path mismatch".into());
    }
    Ok(())
}

pub fn snapshot_matches_applied(step: &Step, snapshot: &Value) -> Result<bool, String> {
    log::info!("Checking GPU reset snapshot against the active profile");
    let old: Snapshot = serde_json::from_value(step.before.clone().ok_or("Missing GPU snapshot")?)
        .map_err(|e| e.to_string())?;
    let current: Snapshot = serde_json::from_value(snapshot.clone()).map_err(|e| e.to_string())?;
    Ok(current.value == desired(&action(step)?, &old)?)
}

pub(crate) fn raw(value: RegValue<'_>) -> RawValue {
    log::info!("Serializing GPU preference");
    RawValue {
        kind: value.vtype as u32,
        bytes: value.bytes.into_owned(),
    }
}

pub(crate) fn reg_type(kind: u32) -> Result<RegType, String> {
    log::info!("Validating registry value type");
    use RegType::*;
    match kind {
        0 => Ok(REG_NONE),
        1 => Ok(REG_SZ),
        2 => Ok(REG_EXPAND_SZ),
        3 => Ok(REG_BINARY),
        4 => Ok(REG_DWORD),
        5 => Ok(REG_DWORD_BIG_ENDIAN),
        6 => Ok(REG_LINK),
        7 => Ok(REG_MULTI_SZ),
        8 => Ok(REG_RESOURCE_LIST),
        9 => Ok(REG_FULL_RESOURCE_DESCRIPTOR),
        10 => Ok(REG_RESOURCE_REQUIREMENTS_LIST),
        11 => Ok(REG_QWORD),
        _ => Err(format!("Unsupported registry value type {kind}")),
    }
}

fn open_key() -> Result<RegKey, String> {
    log::info!("Opening GPU preference key");
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(KEY, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("Cannot open GPU preference key: {e}"))
}

fn read_value(key: &RegKey, path: &str) -> Result<Option<RawValue>, String> {
    log::info!("Reading GPU preference");
    match key.get_raw_value(path) {
        Ok(value) => Ok(Some(raw(value))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read GPU preference: {e}")),
    }
}

fn desired(action: &GpuAction, before: &Snapshot) -> Result<Option<RawValue>, String> {
    log::info!("Preparing GPU preference");
    if let Some(value) = &before.value {
        reg_type(value.kind)?;
    }
    if action.preference == 0 {
        return Ok(None);
    }
    let existing = match &before.value {
        None => String::new(),
        Some(value) => {
            if value.kind != 1 || value.bytes.len() % 2 != 0 {
                return Err("Existing GPU preference is not a valid registry string".into());
            }
            let mut words: Vec<u16> = value
                .bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            while words.last() == Some(&0) {
                words.pop();
            }
            if words.contains(&0) {
                return Err("Existing GPU preference contains an embedded null".into());
            }
            String::from_utf16(&words).map_err(|e| format!("Invalid GPU preference text: {e}"))?
        }
    };
    let mut parts: Vec<&str> = existing
        .split(';')
        .filter(|part| {
            !part.trim().is_empty()
                && !part
                    .split_once('=')
                    .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("GpuPreference"))
        })
        .collect();
    let preference = format!("GpuPreference={}", action.preference);
    parts.push(&preference);
    Ok(Some(raw(format!("{};", parts.join(";")).to_reg_value())))
}

fn write_value(key: &RegKey, path: &str, value: &Option<RawValue>) -> Result<(), String> {
    match value {
        Some(value) => key.set_raw_value(
            path,
            &RegValue {
                vtype: reg_type(value.kind)?,
                bytes: Cow::Owned(value.bytes.clone()),
            },
        ),
        None => match key.delete_value(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        },
    }
    .map_err(|e| format!("Cannot write GPU preference: {e}"))
}

pub struct GpuExecutor;

impl GpuExecutor {
    pub fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        log::info!("Checking applied GPU preference");
        let action = action(step)?;
        let before: Snapshot =
            serde_json::from_value(step.before.clone().ok_or("Missing GPU snapshot")?)
                .map_err(|e| e.to_string())?;
        Ok(read_value(&open_key()?, &action.path)? == desired(&action, &before)?)
    }
}

impl StepExecutor for GpuExecutor {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        log::info!("Capturing original GPU preference");
        let action = action(step)?;
        if !Path::new(&action.path).is_file() {
            return Err("Executable does not exist".into());
        }
        let snapshot = Snapshot {
            value: read_value(&open_key()?, &action.path)?,
        };
        desired(&action, &snapshot)?;
        serde_json::to_value(snapshot).map_err(|e| e.to_string())
    }

    fn apply(&mut self, step: &Step) -> Result<(), String> {
        log::info!("Applying GPU preference");
        let action = action(step)?;
        let before: Snapshot =
            serde_json::from_value(step.before.clone().ok_or("Missing GPU snapshot")?)
                .map_err(|e| e.to_string())?;
        let key = open_key()?;
        if read_value(&key, &action.path)? != before.value {
            return Err("GPU preference changed since the snapshot".into());
        }
        let value = desired(&action, &before)?;
        write_value(&key, &action.path, &value)?;
        if read_value(&key, &action.path)? != value {
            return Err("GPU preference verification failed after writing".into());
        }
        Ok(())
    }

    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        log::info!("Restoring GPU preference");
        let action = action(step)?;
        let before: Snapshot = serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
        let key = open_key()?;
        let current = read_value(&key, &action.path)?;
        if current == before.value {
            return Ok(());
        }
        if current != desired(&action, &before)? {
            return Err("GPU preference was modified externally; refusing to overwrite it".into());
        }
        write_value(&key, &action.path, &before.value)?;
        if read_value(&key, &action.path)? != before.value {
            return Err("GPU preference verification failed after restoring".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{StepRequest, StepStatus, Transaction};
    use serde_json::json;
    use std::path::PathBuf;

    struct FailAfterWrite;
    impl StepExecutor for FailAfterWrite {
        fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
            GpuExecutor.snapshot(step)
        }
        fn apply(&mut self, step: &Step) -> Result<(), String> {
            GpuExecutor.apply(step)?;
            Err("Simulated failure after the registry write".into())
        }
        fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
            GpuExecutor.restore(step, before)
        }
    }

    struct RecoveryGuard(PathBuf);
    impl Drop for RecoveryGuard {
        fn drop(&mut self) {
            if let Err(e) =
                Transaction::open(&self.0).and_then(|mut tx| tx.restore(&mut GpuExecutor))
            {
                eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
            }
        }
    }

    #[test]
    #[ignore = "changes a real Windows registry value for an explicitly provided test executable"]
    fn real_dummy_gpu_preference_recovers_after_a_partial_failure() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let journal = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_TEST_JOURNAL").expect("Set GAME_OPTIMIZER_TEST_JOURNAL"),
        );
        let original = read_value(&open_key().unwrap(), &executable).unwrap();
        let request = StepRequest {
            id: "gpu".into(),
            action: json!({"kind": "gpu", "path": executable, "preference": 2}),
        };
        let _guard;
        let mut tx = Transaction::create(&journal, &[request]).unwrap();
        _guard = RecoveryGuard(journal.clone());
        assert_ne!(
            original,
            desired(
                &action(&tx.journal.steps[0]).unwrap(),
                &Snapshot {
                    value: original.clone()
                }
            )
            .unwrap(),
            "The test must change the value"
        );
        let failure = tx.apply(&mut FailAfterWrite).unwrap_err();
        assert_eq!(failure.code, "APPLY_FAILED");
        assert_eq!(failure.step.as_deref(), Some("gpu"));
        assert_eq!(
            read_value(&open_key().unwrap(), &executable).unwrap(),
            desired(
                &action(&tx.journal.steps[0]).unwrap(),
                &Snapshot {
                    value: original.clone()
                }
            )
            .unwrap()
        );
        drop(tx);
        let mut recovered = Transaction::open(&journal).unwrap();
        assert_eq!(recovered.journal.steps[0].status, StepStatus::Prepared);
        recovered.restore(&mut GpuExecutor).unwrap();
        assert_eq!(
            read_value(&open_key().unwrap(), &executable).unwrap(),
            original
        );
        drop(recovered);
        assert_eq!(
            Transaction::open(&journal).unwrap().journal.steps[0].status,
            StepStatus::Restored
        );
    }

    #[test]
    #[ignore = "changes a real Windows registry value for an explicitly provided test executable"]
    fn real_dummy_gpu_restores_a_preexisting_value() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let journal = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_TEST_JOURNAL").expect("Set GAME_OPTIMIZER_TEST_JOURNAL"),
        );
        let mut baseline_name = journal.as_os_str().to_os_string();
        baseline_name.push(".baseline");
        let baseline = PathBuf::from(baseline_name);
        let original = read_value(&open_key().unwrap(), &executable).unwrap();
        let request = |preference| StepRequest {
            id: "gpu".into(),
            action: json!({"kind": "gpu", "path": executable, "preference": preference}),
        };
        let baseline_guard;
        let mut baseline_tx = Transaction::create(&baseline, &[request(1)]).unwrap();
        baseline_guard = RecoveryGuard(baseline.clone());
        let baseline_value = desired(
            &action(&baseline_tx.journal.steps[0]).unwrap(),
            &Snapshot {
                value: original.clone(),
            },
        )
        .unwrap();
        assert_ne!(
            original,
            baseline_value.clone(),
            "The baseline must change the value"
        );
        baseline_tx.apply(&mut GpuExecutor).unwrap();
        drop(baseline_tx);

        let test_guard;
        let mut tx = Transaction::create(&journal, &[request(0)]).unwrap();
        test_guard = RecoveryGuard(journal.clone());
        tx.apply(&mut GpuExecutor).unwrap();
        assert_eq!(read_value(&open_key().unwrap(), &executable).unwrap(), None);
        tx.restore(&mut GpuExecutor).unwrap();
        assert_eq!(
            read_value(&open_key().unwrap(), &executable).unwrap(),
            baseline_value
        );
        drop(tx);
        drop(test_guard);
        let mut baseline_tx = Transaction::open(&baseline).unwrap();
        baseline_tx.restore(&mut GpuExecutor).unwrap();
        assert_eq!(
            read_value(&open_key().unwrap(), &executable).unwrap(),
            original
        );
        drop(baseline_tx);
        drop(baseline_guard);
    }

    #[test]
    #[ignore = "restores a real GPU preference using an explicitly provided test journal"]
    fn recover_dummy_gpu_journal() {
        let journal = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_TEST_JOURNAL").expect("Set GAME_OPTIMIZER_TEST_JOURNAL"),
        );
        Transaction::open(&journal)
            .unwrap()
            .restore(&mut GpuExecutor)
            .unwrap();
    }

    #[test]
    fn preserves_other_gpu_preferences_and_rejects_invalid_values() {
        let action = GpuAction {
            kind: "gpu".into(),
            path: r"C:\test.exe".into(),
            preference: 2,
        };
        let existing = Snapshot {
            value: Some(raw(
                "VRROptimizeEnable=1;GpuPreference=1;Other=3;".to_reg_value()
            )),
        };
        let actual = desired(&action, &existing).unwrap();
        let expected = Some(raw(
            "VRROptimizeEnable=1;Other=3;GpuPreference=2;".to_reg_value()
        ));
        assert_eq!(actual, expected);
        let invalid = Snapshot {
            value: Some(RawValue {
                kind: 4,
                bytes: vec![0; 4],
            }),
        };
        assert!(desired(&action, &invalid).is_err());
    }

    #[test]
    fn reset_snapshot_must_match_the_active_gpu_value() {
        let step = Step {
            id: "gpu".into(),
            action: json!({"kind":"gpu","path":r"C:\Games\game.exe","preference":2}),
            status: StepStatus::Applied,
            before: Some(json!({"value":null})),
            error: None,
        };
        let applied = json!({"value":raw("GpuPreference=2;".to_reg_value())});
        assert!(snapshot_matches_applied(&step, &applied).unwrap());
        assert!(!snapshot_matches_applied(&step, &json!({"value":null})).unwrap());
    }
}
