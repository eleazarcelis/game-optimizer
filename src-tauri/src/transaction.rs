use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepStatus {
    Planned,
    Prepared,
    Applied,
    Restoring,
    Restored,
    Skipped,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub action: Value,
    pub status: StepStatus,
    pub before: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StepRequest {
    pub id: String,
    pub action: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Journal {
    pub version: u8,
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<Reset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reset {
    pub phase: ResetPhase,
    pub previous_steps: Option<Vec<Step>>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetPhase {
    Applying,
    Committing,
}

pub trait StepExecutor {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String>;
    fn apply(&mut self, step: &Step) -> Result<(), String>;
    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String>;
}

#[derive(Debug)]
pub struct TransactionError {
    pub code: &'static str,
    pub step: Option<String>,
    pub message: String,
}

fn failure(
    code: &'static str,
    step: Option<&str>,
    error: impl std::fmt::Display,
) -> TransactionError {
    log::error!("Transaction {code} at {:?}: {error}", step);
    TransactionError {
        code,
        step: step.map(str::to_owned),
        message: error.to_string(),
    }
}

/// Marker present in executor errors when a value was modified outside the app.
pub const EXTERNAL_CHANGE: &str = "changed externally";

fn external_change(message: &str) -> bool {
    message.contains(EXTERNAL_CHANGE) || message.contains("modified externally")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkippedStep {
    pub step: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApplyWarning {
    pub step: String,
    pub message: String,
    pub recovery_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_conflict: Option<FolderConflict>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderConflict {
    pub target: String,
    pub folders: Vec<String>,
    pub unidentified: bool,
}

pub struct Transaction {
    path: PathBuf,
    pub journal: Journal,
    pub skipped: Vec<SkippedStep>,
    _lock: fs::File,
}

impl Transaction {
    fn lock(path: &Path) -> Result<fs::File, TransactionError> {
        log::info!("Locking transaction journal");
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(lock_path)
            .map_err(|e| failure("JOURNAL_LOCK", None, e))?;
        lock.try_lock_exclusive()
            .map_err(|e| failure("JOURNAL_LOCKED", None, e))?;
        Ok(lock)
    }

    pub fn create(path: &Path, requests: &[StepRequest]) -> Result<Self, TransactionError> {
        log::info!("Creating transaction journal");
        let unique: std::collections::HashSet<_> = requests.iter().map(|step| &step.id).collect();
        if requests.is_empty()
            || unique.len() != requests.len()
            || requests
                .iter()
                .any(|step| step.id.is_empty() || !step.action.is_object())
        {
            return Err(failure(
                "INVALID_STEPS",
                None,
                "Steps must be nonempty and unique",
            ));
        }
        let lock = Self::lock(path)?;
        let journal = Journal {
            version: 1,
            steps: requests
                .iter()
                .map(|request| Step {
                    id: request.id.clone(),
                    action: request.action.clone(),
                    status: StepStatus::Planned,
                    before: None,
                    error: None,
                })
                .collect(),
            reset: None,
        };
        let bytes =
            serde_json::to_vec_pretty(&journal).map_err(|e| failure("JOURNAL_ENCODE", None, e))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| failure("JOURNAL_CREATE", None, e))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| failure("JOURNAL_WRITE", None, e))?;
        Ok(Self {
            path: path.to_owned(),
            journal,
            skipped: Vec::new(),
            _lock: lock,
        })
    }

    pub fn open(path: &Path) -> Result<Self, TransactionError> {
        log::info!("Opening transaction journal");
        let lock = Self::lock(path)?;
        let bytes = fs::read(path).map_err(|e| failure("JOURNAL_READ", None, e))?;
        let journal: Journal =
            serde_json::from_slice(&bytes).map_err(|e| failure("JOURNAL_INVALID", None, e))?;
        let ids: std::collections::HashSet<_> = journal.steps.iter().map(|s| &s.id).collect();
        if journal.version != 1
            || journal.steps.is_empty()
            || ids.len() != journal.steps.len()
            || journal.steps.iter().any(|s| {
                s.id.is_empty()
                    || !s.action.is_object()
                    || (matches!(s.status, StepStatus::Planned | StepStatus::Discarded)
                        && s.before.is_some())
                    || (s.status == StepStatus::Skipped
                        && s.before.as_ref().is_some_and(|before| {
                            before.is_null() || !s.error.as_deref().is_some_and(external_change)
                        }))
                    || (!matches!(
                        s.status,
                        StepStatus::Planned | StepStatus::Skipped | StepStatus::Discarded
                    ) && s.before.as_ref().is_none_or(Value::is_null))
            })
            || journal.reset.as_ref().is_some_and(|reset| {
                reset.previous_steps.as_ref().is_some_and(|steps| {
                    let ids: std::collections::HashSet<_> = steps.iter().map(|s| &s.id).collect();
                    steps.is_empty()
                        || ids.len() != steps.len()
                        || steps.iter().any(|s| {
                            s.id.is_empty()
                                || !s.action.is_object()
                                || s.status != StepStatus::Applied
                                || s.before.as_ref().is_none_or(Value::is_null)
                        })
                }) || (reset.phase == ResetPhase::Committing
                    && journal
                        .steps
                        .iter()
                        .any(|s| s.status != StepStatus::Applied))
            })
        {
            return Err(failure(
                "JOURNAL_INVALID",
                None,
                "Invalid transaction journal",
            ));
        }
        Ok(Self {
            path: path.to_owned(),
            journal,
            skipped: Vec::new(),
            _lock: lock,
        })
    }

    fn save(&self) -> Result<(), TransactionError> {
        log::info!("Persisting transaction journal");
        let dir = self
            .path
            .parent()
            .ok_or_else(|| failure("JOURNAL_PATH", None, "Missing parent directory"))?;
        let bytes = serde_json::to_vec_pretty(&self.journal)
            .map_err(|e| failure("JOURNAL_ENCODE", None, e))?;
        let mut temp =
            tempfile::NamedTempFile::new_in(dir).map_err(|e| failure("JOURNAL_WRITE", None, e))?;
        temp.write_all(&bytes)
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|e| failure("JOURNAL_WRITE", None, e))?;
        temp.persist(&self.path)
            .map_err(|e| failure("JOURNAL_WRITE", None, e))?;
        Ok(())
    }

    pub fn begin_reset(
        &mut self,
        requests: &[StepRequest],
        retain_previous: bool,
    ) -> Result<(), TransactionError> {
        log::info!("Recording irreversible reset intent");
        let unique: std::collections::HashSet<_> = requests.iter().map(|s| &s.id).collect();
        if self.journal.reset.is_some()
            || requests.is_empty()
            || unique.len() != requests.len()
            || requests
                .iter()
                .any(|s| s.id.is_empty() || !s.action.is_object())
            || (retain_previous
                && self
                    .journal
                    .steps
                    .iter()
                    .any(|s| s.status != StepStatus::Applied))
            || (!retain_previous
                && self
                    .journal
                    .steps
                    .iter()
                    .any(|s| s.status != StepStatus::Planned))
        {
            return Err(failure(
                "RESET_INVALID",
                None,
                "Cannot start reset from this journal",
            ));
        }
        let previous_steps = retain_previous.then(|| self.journal.steps.clone());
        self.journal.steps = requests
            .iter()
            .map(|request| Step {
                id: request.id.clone(),
                action: request.action.clone(),
                status: StepStatus::Planned,
                before: None,
                error: None,
            })
            .collect();
        self.journal.reset = Some(Reset {
            phase: ResetPhase::Applying,
            previous_steps,
        });
        self.save()
    }

    pub fn rollback_reset(
        &mut self,
        executor: &mut impl StepExecutor,
    ) -> Result<(), TransactionError> {
        log::info!("Recovering interrupted reset");
        if self
            .journal
            .reset
            .as_ref()
            .is_none_or(|reset| reset.phase != ResetPhase::Applying)
        {
            return Err(failure(
                "RESET_INVALID",
                None,
                "Reset cannot be rolled back",
            ));
        }
        self.restore(executor)?;
        if !self.skipped.is_empty() {
            return Ok(());
        }
        let previous = self
            .journal
            .reset
            .as_mut()
            .and_then(|reset| reset.previous_steps.take());
        if let Some(previous) = previous {
            self.journal.steps = previous;
        }
        self.journal.reset = None;
        self.save()
    }

    pub fn start_reset_commit(&mut self) -> Result<(), TransactionError> {
        log::info!("Committing irreversible reset");
        if self
            .journal
            .steps
            .iter()
            .any(|s| s.status != StepStatus::Applied)
            || self
                .journal
                .reset
                .as_ref()
                .is_none_or(|reset| reset.phase != ResetPhase::Applying)
        {
            return Err(failure(
                "RESET_INVALID",
                None,
                "Reset is not ready to commit",
            ));
        }
        self.journal.reset.as_mut().unwrap().phase = ResetPhase::Committing;
        self.save()
    }

    pub fn finish_reset(&mut self) -> Result<(), TransactionError> {
        log::info!("Finalizing irreversible reset journal");
        if self
            .journal
            .reset
            .as_ref()
            .is_none_or(|reset| reset.phase != ResetPhase::Committing)
        {
            return Err(failure("RESET_INVALID", None, "Reset is not committing"));
        }
        for step in &mut self.journal.steps {
            step.status = StepStatus::Discarded;
            step.before = None;
            step.error = None;
        }
        self.journal.reset = None;
        self.save()
    }

    pub fn apply(&mut self, executor: &mut impl StepExecutor) -> Result<(), TransactionError> {
        self.apply_inner(executor, false).map(|_| ())
    }

    pub fn apply_best_effort(
        &mut self,
        executor: &mut impl StepExecutor,
    ) -> Result<Vec<ApplyWarning>, TransactionError> {
        if self.journal.reset.is_some() {
            return Err(failure(
                "RESET_INVALID",
                None,
                "Best-effort apply cannot reset a profile",
            ));
        }
        self.apply_inner(executor, true)
    }

    fn apply_inner(
        &mut self,
        executor: &mut impl StepExecutor,
        best_effort: bool,
    ) -> Result<Vec<ApplyWarning>, TransactionError> {
        log::info!("Applying transaction");
        if self
            .journal
            .steps
            .iter()
            .any(|s| s.status != StepStatus::Planned)
        {
            return Err(failure(
                "RECOVERY_REQUIRED",
                None,
                "Restore the existing transaction before applying again",
            ));
        }
        let mut warnings = Vec::new();
        for index in 0..self.journal.steps.len() {
            let id = self.journal.steps[index].id.clone();
            let before = match executor.snapshot(&self.journal.steps[index]) {
                Ok(value) if !value.is_null() => value,
                Ok(_) => {
                    if best_effort {
                        let message = "Snapshot cannot be null".to_string();
                        self.journal.steps[index].status = StepStatus::Skipped;
                        self.journal.steps[index].error = Some(message.clone());
                        self.save()?;
                        warnings.push(ApplyWarning {
                            step: id,
                            message,
                            recovery_required: false,
                            folder_conflict: None,
                        });
                        continue;
                    }
                    self.journal.steps[index].error = Some("Snapshot cannot be null".into());
                    self.save()?;
                    return Err(failure(
                        "SNAPSHOT_INVALID",
                        Some(&id),
                        "Snapshot cannot be null",
                    ));
                }
                Err(message) => {
                    if best_effort {
                        log::warn!("Skipping snapshot of {id}: {message}");
                        self.journal.steps[index].status = StepStatus::Skipped;
                        self.journal.steps[index].error = Some(message.clone());
                        self.save()?;
                        warnings.push(ApplyWarning {
                            step: id,
                            message,
                            recovery_required: false,
                            folder_conflict: None,
                        });
                        continue;
                    }
                    self.journal.steps[index].error = Some(message.clone());
                    self.save()?;
                    return Err(failure("SNAPSHOT_FAILED", Some(&id), message));
                }
            };
            self.journal.steps[index].before = Some(before);
            self.journal.steps[index].error = None;
            self.journal.steps[index].status = StepStatus::Prepared;
            self.save()?;
            if let Err(message) = executor.apply(&self.journal.steps[index]) {
                self.journal.steps[index].error = Some(message.clone());
                self.save()?;
                if best_effort {
                    log::warn!("Skipping application of {id}: {message}");
                    // This conflict is returned by the coordinator before it reserves or writes anything.
                    let no_write = self.journal.steps[index]
                        .action
                        .get("coordinated")
                        .and_then(Value::as_bool)
                        == Some(true)
                        && ((id == "defender-path"
                            && message == crate::defender_coordination::OVERLAP)
                            || (id == "defender-process"
                                && message == crate::defender_coordination::PROCESS_OVERLAP));
                    let recovery = if no_write {
                        Ok(())
                    } else {
                        self.journal.steps[index].status = StepStatus::Restoring;
                        self.save()?;
                        let before = self.journal.steps[index]
                            .before
                            .clone()
                            .expect("prepared step has a snapshot");
                        executor.restore(&self.journal.steps[index], &before)
                    };
                    let recovery_required = recovery.is_err();
                    match recovery {
                        Ok(()) => {
                            self.journal.steps[index].status = StepStatus::Skipped;
                            self.journal.steps[index].before = None;
                        }
                        Err(ref recovery_error) => {
                            log::warn!("Recovery of {id} failed: {recovery_error}");
                            self.journal.steps[index].error = Some(recovery_error.clone());
                        }
                    }
                    self.save()?;
                    let message = match recovery {
                        Ok(()) => message,
                        Err(recovery_error) => {
                            format!("{message}; recovery required: {recovery_error}")
                        }
                    };
                    warnings.push(ApplyWarning {
                        step: id,
                        message,
                        recovery_required,
                        folder_conflict: None,
                    });
                    continue;
                }
                return Err(failure("APPLY_FAILED", Some(&id), message));
            }
            self.journal.steps[index].status = StepStatus::Applied;
            self.save()?;
        }
        Ok(warnings)
    }

    pub fn restore(&mut self, executor: &mut impl StepExecutor) -> Result<(), TransactionError> {
        log::info!("Restoring transaction");
        self.skipped.clear();
        for index in (0..self.journal.steps.len()).rev() {
            let step = &self.journal.steps[index];
            if step.status == StepStatus::Planned {
                self.journal.steps[index].status = StepStatus::Skipped;
                self.save()?;
                continue;
            }
            if matches!(step.status, StepStatus::Restored | StepStatus::Discarded)
                || (step.status == StepStatus::Skipped
                    && (self.journal.reset.is_none() || step.before.is_none()))
            {
                continue;
            }
            let id = step.id.clone();
            let before = step
                .before
                .clone()
                .ok_or_else(|| failure("JOURNAL_INVALID", Some(&id), "Missing snapshot"))?;
            if id == "defender-path"
                && step.action.get("coordinated").and_then(Value::as_bool) == Some(true)
                && step.action.get("independent").and_then(Value::as_bool) != Some(true)
                && self
                    .skipped
                    .iter()
                    .any(|failed| failed.step == "defender-process" && failed.retryable)
            {
                let message = "Defender process restoration is pending".to_string();
                log::warn!("Deferring restore of {id}: {message}");
                self.journal.steps[index].status = StepStatus::Restoring;
                self.journal.steps[index].error = Some(message.clone());
                self.save()?;
                self.skipped.push(SkippedStep {
                    step: id,
                    message,
                    retryable: true,
                });
                continue;
            }
            self.journal.steps[index].status = StepStatus::Restoring;
            self.save()?;
            if let Err(message) = executor.restore(&self.journal.steps[index], &before) {
                self.journal.steps[index].error = Some(message.clone());
                let retryable = self.journal.reset.is_some() || !external_change(&message);
                log::warn!("Skipping restore of {id}: {message}");
                if !retryable {
                    self.journal.steps[index].status = StepStatus::Skipped;
                }
                self.save()?;
                self.skipped.push(SkippedStep {
                    step: id,
                    message,
                    retryable,
                });
                continue;
            }
            self.journal.steps[index].error = None;
            self.journal.steps[index].status = StepStatus::Restored;
            self.save()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Fake {
        values: BTreeMap<String, i32>,
        fail_apply: Option<String>,
        fail_restore: Option<String>,
        conflict_restore: Option<String>,
        null_snapshot: Option<String>,
        order: Vec<String>,
    }
    impl StepExecutor for Fake {
        fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
            if self.null_snapshot.as_deref() == Some(&step.id) {
                return Ok(Value::Null);
            }
            Ok(json!({ "value": self.values[&step.id] }))
        }
        fn apply(&mut self, step: &Step) -> Result<(), String> {
            let id = step.id.as_str();
            *self.values.get_mut(id).unwrap() += 10;
            if self.fail_apply.as_deref() == Some(id) {
                return Err("interrupted after write".into());
            }
            Ok(())
        }
        fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
            let id = step.id.as_str();
            self.order.push(id.into());
            if self.conflict_restore.as_deref() == Some(id) {
                return Err("setting changed externally".into());
            }
            if self.fail_restore.as_deref() == Some(id) {
                return Err("restore failed".into());
            }
            self.values
                .insert(id.into(), before["value"].as_i64().unwrap() as i32);
            Ok(())
        }
    }
    fn fake() -> Fake {
        Fake {
            values: [("a".into(), 1), ("b".into(), 2), ("c".into(), 3)].into(),
            ..Fake::default()
        }
    }
    fn ids() -> Vec<StepRequest> {
        ["a", "b", "c"]
            .into_iter()
            .map(|id| StepRequest {
                id: id.into(),
                action: json!({"target": id}),
            })
            .collect()
    }

    #[test]
    fn profile_apply_continues_with_process_after_folder_overlap() {
        struct Overlap {
            calls: Vec<String>,
        }
        impl StepExecutor for Overlap {
            fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
                self.calls.push(format!("snapshot:{}", step.id));
                Ok(json!({"value": 1}))
            }
            fn apply(&mut self, step: &Step) -> Result<(), String> {
                self.calls.push(format!("apply:{}", step.id));
                if step.id == "defender-path" {
                    Err("Defender exclusion overlaps another active profile".into())
                } else {
                    Ok(())
                }
            }
            fn restore(&mut self, step: &Step, _: &Value) -> Result<(), String> {
                self.calls.push(format!("restore:{}", step.id));
                Ok(())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("journal.json");
        let requests: Vec<_> = ["defender-path", "defender-process", "gpu"]
            .iter()
            .map(|id| StepRequest {
                id: (*id).into(),
                action: json!({"coordinated":true}),
            })
            .collect();
        let mut tx = Transaction::create(&file, &requests).unwrap();
        let mut executor = Overlap { calls: Vec::new() };
        let warnings = tx.apply_best_effort(&mut executor).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(tx.journal.steps[0].status, StepStatus::Skipped);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Applied);
        assert_eq!(tx.journal.steps[2].status, StepStatus::Applied);
        assert!(
            executor
                .calls
                .iter()
                .any(|call| call == "apply:defender-process")
        );
        drop(tx);
        let mut tx = Transaction::open(&file).unwrap();
        tx.restore(&mut executor).unwrap();
        assert_eq!(
            executor
                .calls
                .iter()
                .filter(|call| call.starts_with("restore:"))
                .collect::<Vec<_>>(),
            vec!["restore:gpu", "restore:defender-process"]
        );
    }

    #[test]
    fn profile_apply_recovers_partial_writes_and_retains_failed_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("journal.json");
        let mut tx = Transaction::create(&file, &ids()).unwrap();
        let mut executor = fake();
        executor.fail_apply = Some("b".into());
        executor.fail_restore = Some("b".into());
        let warnings = tx.apply_best_effort(&mut executor).unwrap();
        assert!(warnings[0].recovery_required);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Restoring);
        assert_eq!(executor.values["c"], 13);
        drop(tx);
        let mut tx = Transaction::open(&file).unwrap();
        executor.fail_restore = None;
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.values, fake().values);

        let file = dir.path().join("recovered.json");
        let mut tx = Transaction::create(&file, &ids()).unwrap();
        let mut executor = fake();
        executor.fail_apply = Some("b".into());
        let warnings = tx.apply_best_effort(&mut executor).unwrap();
        assert!(!warnings[0].recovery_required);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Skipped);
        assert_eq!(executor.values["b"], 2);
        executor.order.clear();
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.order, vec!["c", "a"]);
    }

    #[test]
    fn profile_apply_skips_failed_snapshot_and_continues() {
        let dir = tempfile::tempdir().unwrap();
        let mut tx = Transaction::create(&dir.path().join("journal.json"), &ids()).unwrap();
        let mut executor = fake();
        executor.null_snapshot = Some("b".into());
        let warnings = tx.apply_best_effort(&mut executor).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Skipped);
        assert_eq!(executor.values["b"], 2);
        assert_eq!(executor.values["c"], 13);
    }

    #[test]
    fn failed_partial_write_is_recoverable_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        let mut executor = fake();
        executor.fail_apply = Some("b".into());
        let error = tx.apply(&mut executor).unwrap_err();
        assert_eq!(
            (error.code, error.step.as_deref()),
            ("APPLY_FAILED", Some("b"))
        );
        assert_eq!(executor.values["b"], 12);
        drop(tx);
        let mut reopened = Transaction::open(&path).unwrap();
        assert_eq!(reopened.journal.steps[1].action, json!({"target": "b"}));
        assert_eq!(reopened.journal.steps[1].status, StepStatus::Prepared);
        assert_eq!(
            reopened.apply(&mut executor).unwrap_err().code,
            "RECOVERY_REQUIRED"
        );
        reopened.restore(&mut executor).unwrap();
        assert_eq!(executor.order, vec!["b", "a"]);
        assert_eq!(executor.values, fake().values);
        drop(reopened);
        assert_eq!(
            Transaction::open(&path).unwrap().journal.steps[0].status,
            StepStatus::Restored
        );
    }

    #[test]
    fn failed_restore_is_recorded_and_can_be_retried() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        let mut executor = fake();
        tx.apply(&mut executor).unwrap();
        executor.fail_restore = Some("b".into());
        tx.restore(&mut executor).unwrap();
        assert_eq!(tx.skipped.len(), 1);
        assert!(tx.skipped[0].retryable);
        assert_eq!(executor.order, vec!["c", "b", "a"]);
        assert_eq!(executor.values["a"], 1);
        drop(tx);
        let mut reopened = Transaction::open(&path).unwrap();
        assert_eq!(reopened.journal.steps[2].status, StepStatus::Restored);
        assert_eq!(reopened.journal.steps[1].status, StepStatus::Restoring);
        assert_eq!(
            reopened.journal.steps[1].error.as_deref(),
            Some("restore failed")
        );
        executor.fail_restore = None;
        reopened.restore(&mut executor).unwrap();
        assert_eq!(executor.values, fake().values);
    }

    #[test]
    fn skipped_conflict_journal_reopens_and_remaining_steps_can_be_recovered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        let mut executor = fake();
        tx.apply(&mut executor).unwrap();
        executor.conflict_restore = Some("c".into());
        executor.fail_restore = Some("b".into());
        tx.restore(&mut executor).unwrap();
        assert_eq!(tx.skipped.len(), 2);
        assert!(!tx.skipped[0].retryable);
        assert!(tx.skipped[1].retryable);
        assert_eq!(tx.journal.steps[2].status, StepStatus::Skipped);
        assert_eq!(tx.journal.steps[2].before, Some(json!({"value": 3})));
        drop(tx);

        let mut reopened = Transaction::open(&path).unwrap();
        executor.fail_restore = None;
        reopened.restore(&mut executor).unwrap();
        assert_eq!(executor.values["a"], 1);
        assert_eq!(executor.values["b"], 2);
        assert_eq!(executor.values["c"], 13);
        drop(reopened);
        let reopened = Transaction::open(&path).unwrap();
        assert_eq!(reopened.journal.steps[2].status, StepStatus::Skipped);
        assert_eq!(reopened.journal.steps[2].before, Some(json!({"value": 3})));
    }

    #[test]
    fn skipped_snapshot_requires_recorded_external_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        tx.journal.steps[0].status = StepStatus::Skipped;
        tx.journal.steps[0].before = Some(json!({"value": 1}));
        tx.save().unwrap();
        drop(tx);
        assert_eq!(
            Transaction::open(&path).err().unwrap().code,
            "JOURNAL_INVALID"
        );
    }

    #[test]
    fn modified_externally_is_skipped_and_other_steps_restore() {
        struct Conflict;
        impl StepExecutor for Conflict {
            fn snapshot(&mut self, _: &Step) -> Result<Value, String> {
                Ok(json!({"value": 1}))
            }
            fn apply(&mut self, _: &Step) -> Result<(), String> {
                Ok(())
            }
            fn restore(&mut self, step: &Step, _: &Value) -> Result<(), String> {
                if step.id == "b" {
                    Err("GPU preference was modified externally; refusing to overwrite it".into())
                } else {
                    Ok(())
                }
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        tx.apply(&mut Conflict).unwrap();
        tx.restore(&mut Conflict).unwrap();
        assert_eq!(tx.skipped.len(), 1);
        assert!(!tx.skipped[0].retryable);
        assert_eq!(tx.journal.steps[0].status, StepStatus::Restored);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Skipped);
        assert_eq!(tx.journal.steps[2].status, StepStatus::Restored);
        drop(tx);
        assert!(Transaction::open(&path).is_ok());
    }

    #[test]
    fn interrupted_reset_keeps_previous_backup_until_all_steps_recover() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut executor = fake();
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        tx.apply(&mut executor).unwrap();
        tx.begin_reset(&ids(), true).unwrap();
        tx.apply(&mut executor).unwrap();
        executor.fail_restore = Some("b".into());
        tx.rollback_reset(&mut executor).unwrap();
        assert_eq!(tx.skipped.len(), 1);
        assert!(tx.journal.reset.is_some());
        assert_eq!(tx.journal.steps[0].status, StepStatus::Restored);
        drop(tx);
        let mut reopened = Transaction::open(&path).unwrap();
        executor.fail_restore = None;
        reopened.rollback_reset(&mut executor).unwrap();
        assert!(reopened.journal.reset.is_none());
        assert_eq!(executor.values["a"], 11);
        assert_eq!(executor.values["b"], 12);
        assert_eq!(executor.values["c"], 13);
    }

    #[test]
    fn coordinated_defender_directory_waits_for_failed_process_restore() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let requests = ["a", "defender-path", "defender-process"]
            .into_iter()
            .map(|id| StepRequest {
                id: id.into(),
                action: json!({"coordinated": id.starts_with("defender-")}),
            })
            .collect::<Vec<_>>();
        let mut executor = Fake {
            values: [
                ("a".into(), 1),
                ("defender-path".into(), 2),
                ("defender-process".into(), 3),
            ]
            .into(),
            ..Fake::default()
        };
        let mut tx = Transaction::create(&path, &requests).unwrap();
        tx.apply(&mut executor).unwrap();
        executor.fail_restore = Some("defender-process".into());
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.order, vec!["defender-process", "a"]);
        assert_eq!(tx.skipped.len(), 2);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Restoring);
        drop(tx);
        let mut tx = Transaction::open(&path).unwrap();
        executor.fail_restore = None;
        executor.order.clear();
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.order, vec!["defender-process", "defender-path"]);
        assert!(tx.skipped.is_empty());
    }

    #[test]
    fn independent_defender_folder_restores_when_process_restore_fails() {
        let dir = tempfile::tempdir().unwrap();
        let requests: Vec<_> = ["defender-path", "defender-process"]
            .into_iter()
            .map(|id| StepRequest {
                id: id.into(),
                action: json!({"coordinated":true,"independent":true}),
            })
            .collect();
        let mut executor = Fake {
            values: [("defender-path".into(), 1), ("defender-process".into(), 2)].into(),
            ..Fake::default()
        };
        let mut tx = Transaction::create(&dir.path().join("journal.json"), &requests).unwrap();
        tx.apply(&mut executor).unwrap();
        executor.fail_restore = Some("defender-process".into());
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.order, vec!["defender-process", "defender-path"]);
        assert_eq!(tx.skipped.len(), 1);
        assert_eq!(tx.journal.steps[0].status, StepStatus::Restored);
        assert_eq!(tx.journal.steps[1].status, StepStatus::Restoring);
    }

    #[test]
    fn journal_does_not_overwrite_an_existing_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        Transaction::create(&path, &ids()).unwrap();
        assert_eq!(
            Transaction::create(&path, &ids()).err().unwrap().code,
            "JOURNAL_CREATE"
        );
        assert_eq!(Transaction::open(&path).unwrap().journal.steps.len(), 3);
    }

    #[test]
    fn locks_a_journal_against_concurrent_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let tx = Transaction::create(&path, &ids()).unwrap();
        assert_eq!(
            Transaction::open(&path).err().unwrap().code,
            "JOURNAL_LOCKED"
        );
        drop(tx);
        assert!(Transaction::open(&path).is_ok());
    }

    #[test]
    fn invalid_snapshot_is_recorded_before_any_write_and_cleared_on_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        let mut executor = fake();
        executor.null_snapshot = Some("a".into());
        assert_eq!(
            tx.apply(&mut executor).unwrap_err().code,
            "SNAPSHOT_INVALID"
        );
        assert_eq!(
            tx.journal.steps[0].error.as_deref(),
            Some("Snapshot cannot be null")
        );
        assert_eq!(executor.values, fake().values);
        executor.null_snapshot = None;
        tx.apply(&mut executor).unwrap();
        assert_eq!(tx.journal.steps[0].error, None);
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.values, fake().values);
    }

    #[test]
    fn interrupted_reset_recovers_the_active_profile_and_its_original_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut executor = fake();
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        tx.apply(&mut executor).unwrap();
        tx.begin_reset(&ids(), true).unwrap();
        executor.fail_apply = Some("b".into());
        assert_eq!(tx.apply(&mut executor).unwrap_err().code, "APPLY_FAILED");
        drop(tx);
        let mut tx = Transaction::open(&path).unwrap();
        executor.fail_apply = None;
        tx.rollback_reset(&mut executor).unwrap();
        assert_eq!(executor.values["a"], 11);
        assert_eq!(executor.values["b"], 12);
        assert!(tx.journal.reset.is_none());
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.values, fake().values);
    }

    #[test]
    fn committed_reset_discards_all_backups_and_cannot_be_rolled_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let mut executor = fake();
        let mut tx = Transaction::create(&path, &ids()).unwrap();
        tx.apply(&mut executor).unwrap();
        tx.begin_reset(&ids(), true).unwrap();
        tx.apply(&mut executor).unwrap();
        tx.start_reset_commit().unwrap();
        drop(tx);
        let mut tx = Transaction::open(&path).unwrap();
        assert_eq!(
            tx.rollback_reset(&mut executor).unwrap_err().code,
            "RESET_INVALID"
        );
        tx.finish_reset().unwrap();
        assert!(tx.journal.reset.is_none());
        assert!(
            tx.journal
                .steps
                .iter()
                .all(|s| s.status == StepStatus::Discarded && s.before.is_none())
        );
        tx.restore(&mut executor).unwrap();
        assert_eq!(executor.values["a"], 21);
        drop(tx);
        let tx = Transaction::open(&path).unwrap();
        assert!(tx.journal.steps.iter().all(|s| s.before.is_none()));
    }
}
