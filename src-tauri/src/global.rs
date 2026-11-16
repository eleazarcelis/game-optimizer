use crate::{
    AppError, error,
    gpu::{RawValue, raw, reg_type},
    power::{self, PowerExecutor},
    transaction::{Step, StepExecutor, StepRequest, StepStatus, Transaction, TransactionError},
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
};
use winreg::{
    RegKey, RegValue,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    types::ToRegValue,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    power: String,
    game_mode: String,
    game_dvr: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    kind: String,
    enabled: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    value: Option<RawValue>,
}

#[derive(Serialize)]
pub struct Status {
    state: &'static str,
    detail: Option<String>,
}

fn action(step: &Step) -> Result<Action, String> {
    log::info!("Validating global transaction action");
    let action: Action = serde_json::from_value(step.action.clone()).map_err(|e| e.to_string())?;
    if !matches!(action.kind.as_str(), "gameMode" | "gameDvr") {
        return Err("Invalid global registry action".into());
    }
    Ok(action)
}

fn location(kind: &str) -> Result<(&'static str, &'static str), String> {
    log::info!("Resolving global registry location");
    match kind {
        "gameMode" => Ok((r"Software\Microsoft\GameBar", "AutoGameModeEnabled")),
        "gameDvr" => Ok((r"System\GameConfigStore", "GameDVR_Enabled")),
        _ => Err("Invalid global registry action".into()),
    }
}

trait Registry {
    fn read(&mut self, kind: &str) -> Result<Option<RawValue>, String>;
    fn write(&mut self, kind: &str, value: &Option<RawValue>) -> Result<(), String>;
}

struct WindowsRegistry;

impl Registry for WindowsRegistry {
    fn read(&mut self, kind: &str) -> Result<Option<RawValue>, String> {
        log::info!("Reading global registry setting {kind}");
        let (path, name) = location(kind)?;
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(path, KEY_READ)
            .map_err(|e| format!("Cannot open {kind} key: {e}"))?;
        match key.get_raw_value(name) {
            Ok(value) => Ok(Some(raw(value))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Cannot read {kind}: {e}")),
        }
    }

    fn write(&mut self, kind: &str, value: &Option<RawValue>) -> Result<(), String> {
        log::info!("Writing global registry setting {kind}");
        let (path, name) = location(kind)?;
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(path, KEY_READ | KEY_WRITE)
            .map_err(|e| format!("Cannot open {kind} key for writing: {e}"))?;
        match value {
            Some(value) => key.set_raw_value(
                name,
                &RegValue {
                    vtype: reg_type(value.kind)?,
                    bytes: Cow::Owned(value.bytes.clone()),
                },
            ),
            None => match key.delete_value(name) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                result => result,
            },
        }
        .map_err(|e| format!("Cannot write {kind}: {e}"))
    }
}

struct Executor<R>(R);
impl<R: Registry> Executor<R> {
    fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        log::info!("Checking applied global registry setting");
        let action = action(step)?;
        Ok(self.0.read(&action.kind)? == Some(raw(u32::from(action.enabled).to_reg_value())))
    }
}
impl<R: Registry> StepExecutor for Executor<R> {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        log::info!("Capturing global setting snapshot");
        let action = action(step)?;
        serde_json::to_value(Snapshot {
            value: self.0.read(&action.kind)?,
        })
        .map_err(|e| e.to_string())
    }

    fn apply(&mut self, step: &Step) -> Result<(), String> {
        log::info!("Applying global registry setting");
        let action = action(step)?;
        let before: Snapshot =
            serde_json::from_value(step.before.clone().ok_or("Missing global snapshot")?)
                .map_err(|e| e.to_string())?;
        if self.0.read(&action.kind)? != before.value {
            return Err("Global setting changed since the snapshot".into());
        }
        let desired = Some(raw(u32::from(action.enabled).to_reg_value()));
        self.0.write(&action.kind, &desired)?;
        if self.0.read(&action.kind)? != desired {
            return Err("Global setting verification failed after writing".into());
        }
        Ok(())
    }

    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        log::info!("Restoring global registry setting");
        let action = action(step)?;
        let before: Snapshot = serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
        let current = self.0.read(&action.kind)?;
        if current == before.value {
            return Ok(());
        }
        if current != Some(raw(u32::from(action.enabled).to_reg_value())) {
            return Err("Global setting changed externally; refusing to overwrite it".into());
        }
        self.0.write(&action.kind, &before.value)?;
        if self.0.read(&action.kind)? != before.value {
            return Err("Global setting verification failed after restoration".into());
        }
        Ok(())
    }
}

struct Combined<R> {
    registry: Executor<R>,
    power: Option<PowerExecutor>,
}
impl<R: Registry> Combined<R> {
    fn new(registry: R) -> Self {
        Self {
            registry: Executor(registry),
            power: None,
        }
    }
    fn power(&mut self) -> Result<&mut PowerExecutor, String> {
        if self.power.is_none() {
            self.power = Some(PowerExecutor::new()?);
        }
        self.power
            .as_mut()
            .ok_or_else(|| "Power executor unavailable".into())
    }
    fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        if step.id == "power" {
            self.power()?.matches_applied(step)
        } else {
            self.registry.matches_applied(step)
        }
    }
}
impl<R: Registry> StepExecutor for Combined<R> {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        if step.id == "power" {
            self.power()?.snapshot(step)
        } else {
            self.registry.snapshot(step)
        }
    }
    fn apply(&mut self, step: &Step) -> Result<(), String> {
        if step.id == "power" {
            self.power()?.apply(step)
        } else {
            self.registry.apply(step)
        }
    }
    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        if step.id == "power" {
            self.power()?.restore(step, before)
        } else {
            self.registry.restore(step, before)
        }
    }
}

fn requests(settings: &Settings) -> Result<Vec<StepRequest>, AppError> {
    log::info!("Validating global settings");
    if !matches!(settings.power.as_str(), "none" | "balanced" | "high")
        || !matches!(settings.game_mode.as_str(), "none" | "on" | "off")
        || !matches!(settings.game_dvr.as_str(), "none" | "on" | "off")
    {
        return Err(error("INVALID_SETTINGS", "Invalid global setting"));
    }
    let mut steps = Vec::new();
    if settings.power != "none" {
        steps.push(StepRequest {
            id: "power".into(),
            action: json!({"kind":"power","scheme":if settings.power == "high" { power::HIGH } else { power::BALANCED },"owner":uuid::Uuid::new_v4()}),
        });
    }
    steps.extend(
        [
            ("gameMode", &settings.game_mode),
            ("gameDvr", &settings.game_dvr),
        ]
        .into_iter()
        .filter(|(_, value)| value.as_str() != "none")
        .map(|(kind, value)| StepRequest {
            id: kind.into(),
            action: json!({"kind":kind,"enabled":value == "on"}),
        }),
    );
    if steps.is_empty() {
        return Err(error("NO_ACTION", "No global actions are enabled"));
    }
    Ok(steps)
}

fn transaction_error(e: TransactionError) -> AppError {
    error(
        e.code,
        match e.step {
            Some(step) => format!("{step}: {}", e.message),
            None => e.message,
        },
    )
}

fn directory(root: &Path) -> Result<PathBuf, AppError> {
    log::info!("Resolving global transaction directory");
    let dir = root.join("global-user");
    fs::create_dir_all(&dir).map_err(|e| error("JOURNAL_DIRECTORY", e))?;
    Ok(dir)
}

fn lock(dir: &Path) -> Result<fs::File, AppError> {
    log::info!("Locking global transaction resource");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(dir.join("resource.lock"))
        .map_err(|e| error("RESOURCE_LOCK", e))?;
    file.try_lock_exclusive()
        .map_err(|e| error("RESOURCE_BUSY", e))?;
    Ok(file)
}

fn active(dir: &Path) -> Result<Option<PathBuf>, AppError> {
    log::info!("Finding active global transaction");
    let mut active = None;
    for entry in fs::read_dir(dir).map_err(|e| error("JOURNAL_READ", e))? {
        let file = entry.map_err(|e| error("JOURNAL_READ", e))?.path();
        if file.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let tx = Transaction::open(&file).map_err(transaction_error)?;
        let mut kinds = std::collections::HashSet::new();
        for step in &tx.journal.steps {
            if step.id == "power" {
                power::validate(step).map_err(|e| error("JOURNAL_INVALID", e))?;
            } else {
                let action = action(step).map_err(|e| error("JOURNAL_INVALID", e))?;
                if step.id != action.kind {
                    return Err(error(
                        "JOURNAL_INVALID",
                        "Global journal step does not match its action",
                    ));
                }
            }
            if !kinds.insert(&step.id) {
                return Err(error(
                    "JOURNAL_INVALID",
                    "Invalid or duplicate global journal step",
                ));
            }
        }
        if tx
            .journal
            .steps
            .iter()
            .any(|s| !matches!(s.status, StepStatus::Restored | StepStatus::Skipped))
        {
            if active.is_some() {
                return Err(error(
                    "JOURNAL_CONFLICT",
                    "Multiple unfinished global transactions",
                ));
            }
            active = Some(file);
        }
    }
    Ok(active)
}

pub fn status(root: &Path) -> Result<Status, AppError> {
    log::info!("Checking global transaction status");
    let dir = directory(root)?;
    let _guard = lock(&dir)?;
    let Some(file) = active(&dir)? else {
        return Ok(Status {
            state: "none",
            detail: None,
        });
    };
    let tx = Transaction::open(&file).map_err(transaction_error)?;
    let detail = tx.journal.steps.iter().find_map(|s| s.error.clone());
    let mut executor = Combined::new(WindowsRegistry);
    for step in &tx.journal.steps {
        if step.status != StepStatus::Applied {
            return Ok(Status {
                state: "recovery",
                detail,
            });
        }
        if !executor
            .matches_applied(step)
            .map_err(|e| error("GLOBAL_CHECK", e))?
        {
            return Ok(Status {
                state: "conflict",
                detail,
            });
        }
    }
    Ok(Status {
        state: "applied",
        detail,
    })
}

pub fn apply(root: &Path, settings: &Settings) -> Result<(), AppError> {
    log::info!("Applying global settings");
    let steps = requests(settings)?;
    let dir = directory(root)?;
    let _guard = lock(&dir)?;
    if active(&dir)?.is_some() {
        return Err(error(
            "RECOVERY_REQUIRED",
            "Restore the existing global transaction first",
        ));
    }
    let file = dir.join(format!("{}.json", uuid::Uuid::new_v4()));
    let mut tx = Transaction::create(&file, &steps).map_err(transaction_error)?;
    let mut executor = Combined::new(WindowsRegistry);
    if let Err(failure) = tx.apply(&mut executor) {
        let restore = tx.restore(&mut executor);
        let mut result = transaction_error(failure);
        result.message.push_str(&match restore {
            Ok(()) => "; previous settings restored".into(),
            Err(e) => format!("; recovery required: {}: {}", e.code, e.message),
        });
        return Err(result);
    }
    Ok(())
}

pub fn restore(root: &Path) -> Result<(), AppError> {
    log::info!("Restoring global settings");
    let dir = directory(root)?;
    let _guard = lock(&dir)?;
    let file =
        active(&dir)?.ok_or_else(|| error("NO_BACKUP", "No active global transaction exists"))?;
    Transaction::open(&file)
        .map_err(transaction_error)?
        .restore(&mut Combined::new(WindowsRegistry))
        .map_err(transaction_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Fake(HashMap<String, Option<RawValue>>);
    impl Registry for Fake {
        fn read(&mut self, kind: &str) -> Result<Option<RawValue>, String> {
            Ok(self.0.get(kind).cloned().flatten())
        }
        fn write(&mut self, kind: &str, value: &Option<RawValue>) -> Result<(), String> {
            self.0.insert(kind.into(), value.clone());
            Ok(())
        }
    }

    #[test]
    fn restores_absent_and_existing_values_and_detects_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            power: "none".into(),
            game_mode: "on".into(),
            game_dvr: "off".into(),
        };
        let mut tx =
            Transaction::create(&dir.path().join("test.json"), &requests(&settings).unwrap())
                .unwrap();
        let original = raw(7u32.to_reg_value());
        let mut executor = Executor(Fake(HashMap::from([(
            "gameMode".into(),
            Some(original.clone()),
        )])));
        tx.apply(&mut executor).unwrap();
        executor
            .0
            .write("gameDvr", &Some(raw(5u32.to_reg_value())))
            .unwrap();
        assert_eq!(
            tx.restore(&mut executor).unwrap_err().code,
            "RESTORE_FAILED"
        );
        assert_eq!(
            executor.0.read("gameMode").unwrap(),
            Some(raw(1u32.to_reg_value()))
        );
        executor
            .0
            .write("gameDvr", &Some(raw(0u32.to_reg_value())))
            .unwrap();
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.0.read("gameMode").unwrap(), Some(original));
        assert_eq!(executor.0.read("gameDvr").unwrap(), None);
    }

    #[test]
    fn rejects_invalid_power_before_any_transaction() {
        let settings = Settings {
            power: "invalid".into(),
            game_mode: "on".into(),
            game_dvr: "off".into(),
        };
        assert_eq!(requests(&settings).unwrap_err().code, "INVALID_SETTINGS");
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            apply(root.path(), &settings).unwrap_err().code,
            "INVALID_SETTINGS"
        );
        assert!(!root.path().join("global-user").exists());
    }

    #[test]
    fn interrupted_global_transaction_blocks_new_changes_until_recovered() {
        let root = tempfile::tempdir().unwrap();
        let settings = Settings {
            power: "none".into(),
            game_mode: "on".into(),
            game_dvr: "off".into(),
        };
        let dir = directory(root.path()).unwrap();
        let path = dir.join("test.json");
        drop(Transaction::create(&path, &requests(&settings).unwrap()).unwrap());
        assert_eq!(status(root.path()).unwrap().state, "recovery");
        assert_eq!(
            apply(root.path(), &settings).unwrap_err().code,
            "RECOVERY_REQUIRED"
        );
        restore(root.path()).unwrap();
        assert_eq!(status(root.path()).unwrap().state, "none");
        assert!(
            Transaction::open(&path)
                .unwrap()
                .journal
                .steps
                .iter()
                .all(|s| s.status == StepStatus::Skipped)
        );
    }

    #[test]
    #[ignore = "temporarily changes Game Mode and Game DVR for the current Windows user"]
    fn real_user_settings_recover_after_partial_failure() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_GLOBAL_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_GLOBAL_TEST_ROOT"),
        );
        fs::create_dir_all(&root).unwrap();
        let path = root.join("global-recovery.json");
        assert!(!path.exists(), "Use a fresh persistent journal path");
        let mut registry = WindowsRegistry;
        let original_mode = registry.read("gameMode").unwrap();
        let original_dvr = registry.read("gameDvr").unwrap();
        let opposite = |value: &Option<RawValue>| {
            if *value == Some(raw(1u32.to_reg_value())) {
                "off"
            } else {
                "on"
            }
        };
        let settings = Settings {
            power: "none".into(),
            game_mode: opposite(&original_mode).into(),
            game_dvr: opposite(&original_dvr).into(),
        };
        struct Guard(PathBuf);
        impl Drop for Guard {
            fn drop(&mut self) {
                if self.0.exists() {
                    if let Err(e) = Transaction::open(&self.0)
                        .and_then(|mut tx| tx.restore(&mut Executor(WindowsRegistry)))
                    {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        struct FailAfterDvr(Executor<WindowsRegistry>);
        impl StepExecutor for FailAfterDvr {
            fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
                self.0.snapshot(step)
            }
            fn apply(&mut self, step: &Step) -> Result<(), String> {
                self.0.apply(step)?;
                if step.id == "gameDvr" {
                    return Err("Simulated interruption after write".into());
                }
                Ok(())
            }
            fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
                self.0.restore(step, before)
            }
        }
        let _guard = Guard(path.clone());
        let mut tx = Transaction::create(&path, &requests(&settings).unwrap()).unwrap();
        assert_eq!(
            tx.apply(&mut FailAfterDvr(Executor(WindowsRegistry)))
                .unwrap_err()
                .code,
            "APPLY_FAILED"
        );
        drop(tx);
        let mut reopened = Transaction::open(&path).unwrap();
        reopened.restore(&mut Executor(WindowsRegistry)).unwrap();
        drop(reopened);
        assert_eq!(registry.read("gameMode").unwrap(), original_mode);
        assert_eq!(registry.read("gameDvr").unwrap(), original_dvr);
    }

    #[test]
    #[ignore = "restores a real global-settings test journal"]
    fn recover_user_settings_journal() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_GLOBAL_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_GLOBAL_TEST_ROOT"),
        );
        Transaction::open(&root.join("global-recovery.json"))
            .unwrap()
            .restore(&mut Executor(WindowsRegistry))
            .unwrap();
    }

    #[test]
    #[ignore = "temporarily changes Game Mode for the current Windows user"]
    fn real_user_service_applies_and_restores() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_GLOBAL_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_GLOBAL_TEST_ROOT"),
        );
        struct Guard<'a>(&'a Path);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Err(e) = restore(self.0) {
                    if e.code != "NO_BACKUP" {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        let mut registry = WindowsRegistry;
        let before = registry.read("gameMode").unwrap();
        let desired = if before == Some(raw(1u32.to_reg_value())) {
            "off"
        } else {
            "on"
        };
        let settings = Settings {
            power: "none".into(),
            game_mode: desired.into(),
            game_dvr: "none".into(),
        };
        assert_eq!(status(&root).unwrap().state, "none");
        let _guard = Guard(&root);
        apply(&root, &settings).unwrap();
        assert_eq!(status(&root).unwrap().state, "applied");
        assert_eq!(
            apply(&root, &settings).unwrap_err().code,
            "RECOVERY_REQUIRED"
        );
        restore(&root).unwrap();
        assert_eq!(status(&root).unwrap().state, "none");
        assert_eq!(registry.read("gameMode").unwrap(), before);
    }

    #[test]
    #[ignore = "restores a real global-settings service test journal"]
    fn recover_user_service_journal() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_GLOBAL_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_GLOBAL_TEST_ROOT"),
        );
        restore(&root).unwrap();
    }
}
