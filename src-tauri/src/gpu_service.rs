use crate::{
    AppError,
    cpu::{self, CpuExecutor},
    defender::{self, DefenderExecutor},
    error, executable,
    gpu::{self, GpuExecutor},
    transaction::{
        ResetPhase, Step, StepExecutor, StepRequest, StepStatus, Transaction, TransactionError,
    },
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProfileActions {
    pub cpu: String,
    pub gpu: String,
    pub defender: bool,
}

#[derive(Serialize)]
pub struct GpuStatus {
    pub state: &'static str,
    pub detail: Option<String>,
    #[serde(rename = "appliedProfile")]
    pub applied_profile: Option<ProfileActions>,
}

fn applied_profile(steps: &[Step]) -> Option<ProfileActions> {
    let mut profile = ProfileActions {
        cpu: "none".into(),
        gpu: "none".into(),
        defender: false,
    };
    let mut defender_path = false;
    let mut defender_process = false;
    for step in steps {
        match step.id.as_str() {
            "cpu" => {
                profile.cpu = match step.action.get("priority")?.as_u64()? {
                    1 => "low",
                    3 => "high",
                    _ => return None,
                }
                .into();
            }
            "gpu" => {
                profile.gpu = match step.action.get("preference")?.as_u64()? {
                    1 => "balanced",
                    2 => "high",
                    _ => return None,
                }
                .into();
            }
            "defender-path" | "defender-process" => {
                if step.action.get("enabled")?.as_bool()? != true {
                    return None;
                }
                if step.id == "defender-path" {
                    defender_path = true;
                } else {
                    defender_process = true;
                }
            }
            _ => return None,
        }
    }
    if defender_path != defender_process {
        return None;
    }
    profile.defender = defender_path;
    Some(profile)
}

fn transaction_error(e: TransactionError) -> AppError {
    let detail = match e.step {
        Some(step) => format!("{step}: {}", e.message),
        None => e.message,
    };
    error(e.code, detail)
}

fn requests(
    path: &str,
    profile: &ProfileActions,
    defaults: bool,
) -> Result<Vec<StepRequest>, AppError> {
    log::info!("Validating profile actions");
    if !matches!(profile.cpu.as_str(), "none" | "low" | "high")
        || !matches!(profile.gpu.as_str(), "none" | "balanced" | "high")
    {
        return Err(error("INVALID_PROFILE", "Invalid profile action"));
    }
    if defaults && profile.defender {
        return Err(error(
            "INVALID_PROFILE",
            "Defender exclusions cannot be removed through defaults; restore the active transaction",
        ));
    }
    validate_path(path)?;
    let mut steps = Vec::new();
    if profile.defender {
        let folder = Path::new(path)
            .parent()
            .and_then(Path::to_str)
            .ok_or_else(|| error("INVALID_EXECUTABLE", "Executable folder is invalid"))?;
        let owner = uuid::Uuid::new_v4();
        for (kind, target) in [("defender-path", folder), ("defender-process", path)] {
            defender::validate_target(kind, target).map_err(|e| error("INVALID_EXECUTABLE", e))?;
            steps.push(StepRequest { id:kind.into(), action:json!({"kind":kind,"path":path,"target":target,"owner":owner,"enabled":true,"coordinated":true}) });
        }
    }
    if profile.cpu != "none" {
        crate::privileged::cpu_name(path).map_err(|e| error("INVALID_EXECUTABLE", e))?;
        steps.push(StepRequest { id:"cpu".into(), action:json!({"kind":"cpu","path":path,"priority":if defaults {0} else if profile.cpu == "high" {3} else {1},"owner":uuid::Uuid::new_v4()}) });
    }
    if profile.gpu != "none" {
        steps.push(StepRequest { id:"gpu".into(), action:json!({"kind":"gpu","path":path,"preference":if defaults {0} else if profile.gpu == "high" {2} else {1}}) });
    }
    if steps.is_empty() && !defaults {
        return Err(error("NO_ACTION", "The profile has no enabled actions"));
    }
    Ok(steps)
}

struct Executor;
impl Executor {
    fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        match step.id.as_str() {
            "gpu" => GpuExecutor.matches_applied(step),
            "cpu" => CpuExecutor.matches_applied(step),
            "defender-path" | "defender-process" => DefenderExecutor.matches_applied(step),
            _ => Err("Invalid profile action".into()),
        }
    }
}
impl StepExecutor for Executor {
    fn snapshot(&mut self, step: &Step) -> Result<serde_json::Value, String> {
        match step.id.as_str() {
            "gpu" => GpuExecutor.snapshot(step),
            "cpu" => CpuExecutor.snapshot(step),
            "defender-path" | "defender-process" => DefenderExecutor.snapshot(step),
            _ => Err("Invalid profile action".into()),
        }
    }
    fn apply(&mut self, step: &Step) -> Result<(), String> {
        match step.id.as_str() {
            "gpu" => GpuExecutor.apply(step),
            "cpu" => CpuExecutor.apply(step),
            "defender-path" | "defender-process" => DefenderExecutor.apply(step),
            _ => Err("Invalid profile action".into()),
        }
    }
    fn restore(&mut self, step: &Step, before: &serde_json::Value) -> Result<(), String> {
        match step.id.as_str() {
            "gpu" => GpuExecutor.restore(step, before),
            "cpu" => CpuExecutor.restore(step, before),
            "defender-path" | "defender-process" => DefenderExecutor.restore(step, before),
            _ => Err("Invalid profile action".into()),
        }
    }
}

struct ResetExecutor {
    previous: Vec<Step>,
}

impl StepExecutor for ResetExecutor {
    fn snapshot(&mut self, step: &Step) -> Result<serde_json::Value, String> {
        log::info!("Capturing reset snapshot");
        let previous = self.previous.iter().find(|old| old.id == step.id);
        if let Some(old) = previous {
            if !Executor.matches_applied(old)? {
                return Err("Active setting or reservation changed before reset".into());
            }
        }
        let snapshot = Executor.snapshot(step)?;
        if let Some(old) = previous {
            let matches = match old.id.as_str() {
                "cpu" => cpu::snapshot_matches_applied(old, &snapshot)?,
                "gpu" => gpu::snapshot_matches_applied(old, &snapshot)?,
                _ => return Err("Invalid previous profile action".into()),
            };
            if !matches {
                return Err("Active setting changed while taking reset snapshot".into());
            }
        }
        Ok(snapshot)
    }

    fn apply(&mut self, step: &Step) -> Result<(), String> {
        Executor.apply(step)
    }

    fn restore(&mut self, step: &Step, before: &serde_json::Value) -> Result<(), String> {
        Executor.restore(step, before)
    }
}

fn validate_path(path: &str) -> Result<(), AppError> {
    let target = Path::new(path);
    if !target.is_absolute()
        || !target
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
    {
        return Err(error(
            "INVALID_EXECUTABLE",
            "An absolute executable path is required",
        ));
    }
    Ok(())
}

fn resource_dir(root: &Path, path: &str) -> Result<PathBuf, AppError> {
    log::info!("Resolving GPU transaction directory");
    validate_path(path)?;
    let path_key = normalized(path);
    let hash = format!("{:x}", Sha256::digest(path_key.as_bytes()));
    let dir = root.join("gpu").join(hash);
    fs::create_dir_all(&dir).map_err(|e| error("JOURNAL_DIRECTORY", e))?;
    Ok(dir)
}

fn normalized(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn lock(dir: &Path) -> Result<fs::File, AppError> {
    log::info!("Locking GPU transaction resource");
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

fn find_active(dir: &Path, path: &str) -> Result<Option<PathBuf>, AppError> {
    log::info!("Finding active GPU transaction");
    let mut active = None;
    for entry in fs::read_dir(dir).map_err(|e| error("JOURNAL_READ", e))? {
        let entry = entry.map_err(|e| error("JOURNAL_READ", e))?;
        let file = entry.path();
        if file.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let tx = Transaction::open(&file).map_err(transaction_error)?;
        let previous = tx
            .journal
            .reset
            .as_ref()
            .and_then(|reset| reset.previous_steps.as_ref());
        for step in tx
            .journal
            .steps
            .iter()
            .chain(previous.into_iter().flatten())
        {
            let result = match step.id.as_str() {
                "gpu" => gpu::validate(step, path),
                "cpu" => cpu::validate(step, path),
                "defender-path" | "defender-process" => defender::validate(step, path),
                _ => Err("Unknown profile journal step".into()),
            };
            result.map_err(|e| error("JOURNAL_INVALID", e))?;
        }
        validate_coordinated_defender(&tx.journal.steps)?;
        validate_reset(&tx)?;
        if tx.journal.reset.is_some()
            || tx.journal.steps.iter().any(|s| {
                !matches!(
                    s.status,
                    StepStatus::Restored | StepStatus::Skipped | StepStatus::Discarded
                )
            })
        {
            if active.is_some() {
                return Err(error(
                    "JOURNAL_CONFLICT",
                    "Multiple unfinished GPU transactions require inspection",
                ));
            }
            active = Some(file);
        }
    }
    Ok(active)
}

fn validate_coordinated_defender(steps: &[Step]) -> Result<(), AppError> {
    let coordinated: Vec<_> = steps
        .iter()
        .filter(|step| {
            step.action
                .get("coordinated")
                .and_then(|value| value.as_bool())
                == Some(true)
        })
        .collect();
    if coordinated.is_empty() {
        return Ok(());
    }
    if coordinated.len() != 2
        || coordinated[0].id != "defender-path"
        || coordinated[1].id != "defender-process"
        || coordinated[0].action.get("owner") != coordinated[1].action.get("owner")
        || coordinated
            .iter()
            .any(|step| step.action.get("enabled").and_then(|value| value.as_bool()) != Some(true))
    {
        return Err(error(
            "JOURNAL_INVALID",
            "Invalid coordinated Defender transaction",
        ));
    }
    Ok(())
}

fn validate_reset(tx: &Transaction) -> Result<(), AppError> {
    let Some(reset) = &tx.journal.reset else {
        return Ok(());
    };
    let previous = reset.previous_steps.as_deref().unwrap_or(&[]);
    if previous
        .iter()
        .any(|s| !matches!(s.id.as_str(), "cpu" | "gpu"))
        || previous
            .iter()
            .any(|s| !tx.journal.steps.iter().any(|current| current.id == s.id))
    {
        return Err(error(
            "JOURNAL_INVALID",
            "Reset does not cover its previous profile",
        ));
    }
    for step in &tx.journal.steps {
        match step.id.as_str() {
            "gpu" if step.action.get("preference").and_then(|v| v.as_u64()) == Some(0) => {}
            "cpu" if step.action.get("priority").and_then(|v| v.as_u64()) == Some(0) => {
                let old = previous.iter().find(|s| s.id == "cpu");
                let retained = step
                    .action
                    .get("retain_reservation_on_restore")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if retained != old.is_some()
                    || old.is_some_and(|s| s.action.get("owner") != step.action.get("owner"))
                {
                    return Err(error(
                        "JOURNAL_INVALID",
                        "Reset CPU reservation does not match the previous profile",
                    ));
                }
            }
            _ => return Err(error("JOURNAL_INVALID", "Invalid reset action")),
        }
    }
    Ok(())
}

pub fn status(root: &Path, path: &str) -> Result<GpuStatus, AppError> {
    let can_verify_defender = unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() } != 0;
    status_with(root, path, can_verify_defender, |step| {
        Executor.matches_applied(step)
    })
}

fn status_with(
    root: &Path,
    path: &str,
    can_verify_defender: bool,
    mut matches_applied: impl FnMut(&Step) -> Result<bool, String>,
) -> Result<GpuStatus, AppError> {
    log::info!("Getting GPU transaction status");
    let dir = resource_dir(root, path)?;
    let _guard = lock(&dir)?;
    let Some(file) = find_active(&dir, path)? else {
        return Ok(GpuStatus {
            state: "none",
            detail: None,
            applied_profile: None,
        });
    };
    let tx = Transaction::open(&file).map_err(transaction_error)?;
    let mut detail = tx.journal.steps.iter().find_map(|s| s.error.clone());
    if let Some(reset) = &tx.journal.reset {
        return Ok(GpuStatus {
            state: if reset.phase == ResetPhase::Committing {
                "resetCommit"
            } else {
                "recovery"
            },
            detail,
            applied_profile: None,
        });
    }
    let mut state = "applied";
    let mut defender_unverified = false;
    if tx
        .journal
        .steps
        .iter()
        .any(|step| step.status != StepStatus::Applied)
    {
        state = "recovery";
    }
    for step in &tx.journal.steps {
        if state == "recovery" {
            break;
        }
        let is_defender = matches!(step.id.as_str(), "defender-path" | "defender-process");
        if is_defender && !can_verify_defender {
            defender_unverified = true;
            continue;
        }
        match matches_applied(step) {
            Ok(true) => {}
            Ok(false) => {
                state = "conflict";
                break;
            }
            Err(message) if is_defender => {
                log::error!("Defender status verification failed: {message}");
                defender_unverified = true;
                detail = Some(message);
            }
            Err(message) => return Err(error("ACTION_CHECK", message)),
        }
    }
    if state == "applied"
        && tx.journal.steps.iter().any(|s| {
            s.action.get("preference").and_then(|v| v.as_u64()) == Some(0)
                || s.action.get("priority").and_then(|v| v.as_u64()) == Some(0)
                || s.action.get("enabled").and_then(|v| v.as_bool()) == Some(false)
        })
    {
        state = "defaults";
    }
    if matches!(state, "applied" | "defaults") && defender_unverified {
        state = "unverified";
    }
    let applied_profile = if matches!(state, "applied" | "unverified") {
        applied_profile(&tx.journal.steps)
    } else {
        None
    };
    Ok(GpuStatus {
        state,
        detail,
        applied_profile,
    })
}

pub fn apply(
    root: &Path,
    path: &str,
    profile: &ProfileActions,
    defaults: bool,
) -> Result<(), AppError> {
    log::info!("Applying supported profile actions");
    let requests = requests(path, profile, defaults)?;
    executable(path)?;
    let dir = resource_dir(root, path)?;
    let _guard = lock(&dir)?;
    let active = find_active(&dir, path)?;
    if defaults {
        return reset(&dir, path, requests, active);
    }
    if active.is_some() {
        return Err(error(
            "RECOVERY_REQUIRED",
            "Restore the existing GPU transaction first",
        ));
    }
    let file = dir.join(format!("{}.json", uuid::Uuid::new_v4()));
    let mut tx = Transaction::create(&file, &requests).map_err(transaction_error)?;
    if let Err(failure) = tx.apply(&mut Executor) {
        let restore = tx.restore(&mut Executor);
        let mut result = transaction_error(failure);
        result.message.push_str(&match restore {
            Ok(()) => "; previous state restored".into(),
            Err(e) => format!("; recovery required: {}: {}", e.code, e.message),
        });
        return Err(result);
    }
    Ok(())
}

fn finish_reset(tx: &mut Transaction) -> Result<(), AppError> {
    log::info!("Finishing irreversible profile reset");
    for step in &tx.journal.steps {
        if step.id == "cpu" {
            CpuExecutor
                .finish_reset(step)
                .map_err(|e| error("RESET_FINALIZE", e))?;
        }
    }
    tx.finish_reset().map_err(transaction_error)
}

fn reset(
    dir: &Path,
    path: &str,
    mut requests: Vec<StepRequest>,
    active: Option<PathBuf>,
) -> Result<(), AppError> {
    log::info!("Resetting profile settings without a restorable backup");
    if active.is_none() && requests.is_empty() {
        return Err(error("NO_ACTION", "There are no enabled actions to reset"));
    }
    let retain_previous = active.is_some();
    let mut tx = if let Some(file) = active {
        let tx = Transaction::open(&file).map_err(transaction_error)?;
        if tx.journal.reset.is_some()
            || tx
                .journal
                .steps
                .iter()
                .any(|s| s.status != StepStatus::Applied)
        {
            return Err(error(
                "RECOVERY_REQUIRED",
                "Recover the unfinished transaction before resetting",
            ));
        }
        for step in &tx.journal.steps {
            if !matches!(step.id.as_str(), "cpu" | "gpu") {
                return Err(error(
                    "RECOVERY_REQUIRED",
                    "Restore the existing Defender transaction before resetting",
                ));
            }
            if !Executor
                .matches_applied(step)
                .map_err(|e| error("ACTION_CHECK", e))?
            {
                return Err(error(
                    "ACTION_CONFLICT",
                    "An active setting or reservation changed externally",
                ));
            }
            let mut action = step.action.clone();
            if step.id == "cpu" {
                action["priority"] = json!(0);
                action["retain_reservation_on_restore"] = json!(true);
            } else {
                action["preference"] = json!(0);
            }
            if let Some(existing) = requests.iter_mut().find(|request| request.id == step.id) {
                existing.action = action;
            } else {
                requests.push(StepRequest {
                    id: step.id.clone(),
                    action,
                });
            }
        }
        tx
    } else {
        let file = dir.join(format!("{}.json", uuid::Uuid::new_v4()));
        Transaction::create(&file, &requests).map_err(transaction_error)?
    };
    for request in &requests {
        if !request
            .action
            .get("path")
            .and_then(|v| v.as_str())
            .is_some_and(|p| p.eq_ignore_ascii_case(path))
        {
            return Err(error(
                "JOURNAL_INVALID",
                "Reset path does not match executable",
            ));
        }
    }
    tx.begin_reset(&requests, retain_previous)
        .map_err(transaction_error)?;
    let previous = tx
        .journal
        .reset
        .as_ref()
        .and_then(|reset| reset.previous_steps.clone())
        .unwrap_or_default();
    if let Err(failure) = tx.apply(&mut ResetExecutor { previous }) {
        let recovery = tx.rollback_reset(&mut Executor);
        let mut result = transaction_error(failure);
        result.message.push_str(&match recovery {
            Ok(()) => "; previous state restored".into(),
            Err(e) => format!("; recovery required: {}: {}", e.code, e.message),
        });
        return Err(result);
    }
    for step in &tx.journal.steps {
        if !Executor
            .matches_applied(step)
            .map_err(|e| error("ACTION_CHECK", e))?
        {
            return Err(error(
                "ACTION_CONFLICT",
                "A setting changed before reset could be committed; recovery is required",
            ));
        }
    }
    tx.start_reset_commit().map_err(transaction_error)?;
    finish_reset(&mut tx)
}

pub fn restore(root: &Path, path: &str) -> Result<(), AppError> {
    log::info!("Restoring GPU transaction");
    let dir = resource_dir(root, path)?;
    let _guard = lock(&dir)?;
    let file = find_active(&dir, path)?
        .ok_or_else(|| error("NO_BACKUP", "No active GPU transaction exists"))?;
    let mut tx = Transaction::open(&file).map_err(transaction_error)?;
    if let Some(reset) = &tx.journal.reset {
        if reset.phase == ResetPhase::Committing {
            return finish_reset(&mut tx);
        }
        return tx.rollback_reset(&mut Executor).map_err(transaction_error);
    }
    tx.restore(&mut Executor).map_err(transaction_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_or_empty_profiles_are_rejected_before_any_changes() {
        let profile = ProfileActions {
            cpu: "bad".into(),
            gpu: "high".into(),
            defender: false,
        };
        assert_eq!(
            requests(r"C:\Games\Demo.exe", &profile, false)
                .err()
                .unwrap()
                .code,
            "INVALID_PROFILE"
        );
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: false,
        };
        assert_eq!(
            requests(r"C:\Games\Demo.exe", &profile, false)
                .err()
                .unwrap()
                .code,
            "NO_ACTION"
        );
    }

    #[test]
    fn defender_only_profile_reserves_both_resources_and_defaults_rejects_it() {
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: true,
        };
        let steps = requests(r"C:\Games\Demo.exe", &profile, false).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].id, "defender-path");
        assert_eq!(steps[0].action["target"], r"C:\Games");
        assert_eq!(steps[1].id, "defender-process");
        assert_eq!(steps[1].action["target"], r"C:\Games\Demo.exe");
        assert_eq!(steps[0].action["owner"], steps[1].action["owner"]);
        assert!(steps.iter().all(|step| step.action["coordinated"] == true));
        assert_eq!(
            requests(r"C:\Games\Demo.exe", &profile, true)
                .err()
                .unwrap()
                .code,
            "INVALID_PROFILE"
        );
    }

    #[test]
    fn coordinated_journal_requires_matching_resource_pair() {
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: true,
        };
        let requests = requests(r"C:\Games\Demo.exe", &profile, false).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let tx = Transaction::create(&dir.path().join("tx.json"), &requests).unwrap();
        let mut steps = tx.journal.steps.clone();
        assert!(validate_coordinated_defender(&steps).is_ok());
        steps[1].action["owner"] = json!(uuid::Uuid::new_v4());
        assert_eq!(
            validate_coordinated_defender(&steps).err().unwrap().code,
            "JOURNAL_INVALID"
        );
        assert_eq!(
            validate_coordinated_defender(&steps[..1])
                .err()
                .unwrap()
                .code,
            "JOURNAL_INVALID"
        );
    }

    #[test]
    fn resource_lock_and_unfinished_journal_prevent_overlapping_transactions() {
        let root = tempfile::tempdir().unwrap();
        let path = r"C:\Games\Demo.exe";
        let dir = resource_dir(root.path(), path).unwrap();
        assert_eq!(
            dir,
            resource_dir(root.path(), r"c:\games\demo.EXE").unwrap()
        );
        let guard = lock(&dir).unwrap();
        assert_eq!(lock(&dir).unwrap_err().code, "RESOURCE_BUSY");
        drop(guard);
        let request = StepRequest {
            id: "gpu".into(),
            action: json!({"kind":"gpu","path":path,"preference":2}),
        };
        let file = dir.join("test.json");
        drop(Transaction::create(&file, &[request]).unwrap());
        assert_eq!(find_active(&dir, path).unwrap(), Some(file.clone()));
        restore(root.path(), path).unwrap();
        assert!(find_active(&dir, path).unwrap().is_none());
        assert_eq!(
            Transaction::open(&file).unwrap().journal.steps[0].status,
            StepStatus::Skipped
        );
    }

    #[test]
    fn defender_journal_status_does_not_claim_verification_or_prompt_on_load() {
        let root = tempfile::tempdir().unwrap();
        let path = r"C:\Games\Demo.exe";
        let dir = resource_dir(root.path(), path).unwrap();
        let file = dir.join("test.json");
        let owner = uuid::Uuid::new_v4();
        let journal = json!({"version":1,"steps":[{
            "id":"defender-path","action":{"kind":"defender-path","path":path,"target":r"C:\Games","owner":owner,"enabled":true},
            "status":"Applied","before":{"excluded":false},"error":null
        }]});
        fs::write(file, serde_json::to_vec(&journal).unwrap()).unwrap();
        let result = status_with(root.path(), path, false, |_| {
            panic!("Must not query Defender")
        })
        .unwrap();
        assert_eq!(result.state, "unverified");
        assert_eq!(result.applied_profile, None);
    }

    #[test]
    fn reconstructs_applied_actions_from_existing_journals() {
        let root = tempfile::tempdir().unwrap();
        let path = r"C:\Games\Demo.exe";
        for profile in [
            ProfileActions {
                cpu: "high".into(),
                gpu: "balanced".into(),
                defender: false,
            },
            ProfileActions {
                cpu: "none".into(),
                gpu: "none".into(),
                defender: true,
            },
            ProfileActions {
                cpu: "low".into(),
                gpu: "high".into(),
                defender: true,
            },
        ] {
            let requests = requests(path, &profile, false).unwrap();
            let tx = Transaction::create(
                &root.path().join(format!("{}.json", uuid::Uuid::new_v4())),
                &requests,
            )
            .unwrap();
            assert_eq!(applied_profile(&tx.journal.steps), Some(profile));
        }
        let defaults = ProfileActions {
            cpu: "high".into(),
            gpu: "none".into(),
            defender: false,
        };
        let requests = requests(path, &defaults, true).unwrap();
        let tx = Transaction::create(&root.path().join("defaults.json"), &requests).unwrap();
        assert_eq!(applied_profile(&tx.journal.steps), None);
    }

    #[test]
    fn unverified_defender_status_exposes_the_applied_profile_without_claiming_verification() {
        let root = tempfile::tempdir().unwrap();
        let path = r"C:\Games\Demo.exe";
        let dir = resource_dir(root.path(), path).unwrap();
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: true,
        };
        let requests = requests(path, &profile, false).unwrap();
        let mut tx = Transaction::create(&dir.join("test.json"), &requests).unwrap();
        for step in &mut tx.journal.steps {
            step.status = StepStatus::Applied;
            step.before = Some(json!({"excluded":false}));
        }
        fs::write(
            dir.join("test.json"),
            serde_json::to_vec(&tx.journal).unwrap(),
        )
        .unwrap();
        drop(tx);
        let result = status_with(root.path(), path, false, |_| {
            panic!("Must not query Defender")
        })
        .unwrap();
        assert_eq!(result.state, "unverified");
        assert_eq!(result.applied_profile, Some(profile));
        assert!(serde_json::to_value(result).unwrap()["appliedProfile"]["defender"] == true);
    }

    #[test]
    fn defender_status_distinguishes_verified_conflicts_and_query_failures() {
        let root = tempfile::tempdir().unwrap();
        let path = r"C:\Games\Demo.exe";
        let dir = resource_dir(root.path(), path).unwrap();
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: true,
        };
        let requests = requests(path, &profile, false).unwrap();
        let file = dir.join("test.json");
        let mut tx = Transaction::create(&file, &requests).unwrap();
        for step in &mut tx.journal.steps {
            step.status = StepStatus::Applied;
            step.before = Some(json!({"excluded": false}));
        }
        fs::write(&file, serde_json::to_vec(&tx.journal).unwrap()).unwrap();
        drop(tx);

        let mut checked = Vec::new();
        let result = status_with(root.path(), path, true, |step| {
            checked.push(step.id.clone());
            Ok(true)
        })
        .unwrap();
        assert_eq!(checked, ["defender-path", "defender-process"]);
        assert_eq!(result.state, "applied");
        assert_eq!(result.applied_profile, Some(profile));

        let result = status_with(root.path(), path, true, |step| {
            Ok(step.id != "defender-process")
        })
        .unwrap();
        assert_eq!(result.state, "conflict");
        assert_eq!(result.applied_profile, None);

        let result = status_with(root.path(), path, true, |_| {
            Err("Defender unavailable".into())
        })
        .unwrap();
        assert_eq!(result.state, "unverified");
        assert_eq!(result.detail.as_deref(), Some("Defender unavailable"));
        assert!(result.applied_profile.is_some());

        let mut tx = Transaction::open(&file).unwrap();
        tx.journal.steps[1].status = StepStatus::Restoring;
        fs::write(&file, serde_json::to_vec(&tx.journal).unwrap()).unwrap();
        drop(tx);
        let result = status_with(root.path(), path, true, |_| {
            panic!("Recovery must take priority")
        })
        .unwrap();
        assert_eq!(result.state, "recovery");
    }

    #[test]
    #[ignore = "changes a real Windows registry value for an explicitly provided test executable"]
    fn real_dummy_service_applies_and_restores() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_SERVICE_ROOT").expect("Set GAME_OPTIMIZER_SERVICE_ROOT"),
        );
        struct Guard<'a>(&'a Path, &'a str);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Err(e) = restore(self.0, self.1) {
                    if e.code != "NO_BACKUP" {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        let guard = Guard(&root, &executable);
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "high".into(),
            defender: false,
        };
        apply(&root, &executable, &profile, false).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "applied");
        assert_eq!(
            apply(&root, &executable, &profile, false).unwrap_err().code,
            "RECOVERY_REQUIRED"
        );
        restore(&root, &executable).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        drop(guard);
    }

    #[test]
    #[ignore = "requires UAC and changes CPU and GPU settings for a disposable executable"]
    fn real_cpu_gpu_profile_applies_and_restores() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_SERVICE_ROOT").expect("Set GAME_OPTIMIZER_SERVICE_ROOT"),
        );
        let name = crate::privileged::cpu_name(&executable).unwrap();
        let before_cpu = crate::privileged::read_cpu(&name).unwrap();
        assert!(
            before_cpu.is_none(),
            "Use a unique disposable executable name"
        );
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        struct Guard<'a>(&'a Path, &'a str);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Err(e) = restore(self.0, self.1) {
                    if e.code != "NO_BACKUP" {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        let _guard = Guard(&root, &executable);
        let profile = ProfileActions {
            cpu: "high".into(),
            gpu: "high".into(),
            defender: false,
        };
        apply(&root, &executable, &profile, false).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "applied");
        assert_ne!(crate::privileged::read_cpu(&name).unwrap(), before_cpu);
        restore(&root, &executable).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        assert_eq!(crate::privileged::read_cpu(&name).unwrap(), before_cpu);
        apply(&root, &executable, &profile, true).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        assert_eq!(restore(&root, &executable).unwrap_err().code, "NO_BACKUP");
        assert_eq!(crate::privileged::read_cpu(&name).unwrap(), before_cpu);
    }

    #[test]
    #[ignore = "changes GPU preferences for an explicitly provided disposable executable"]
    fn real_dummy_reset_discards_active_backup() {
        use winreg::{
            RegKey,
            enums::{HKEY_CURRENT_USER, KEY_READ},
        };
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_SERVICE_ROOT").expect("Set GAME_OPTIMIZER_SERVICE_ROOT"),
        );
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(r"Software\Microsoft\DirectX\UserGpuPreferences", KEY_READ)
            .unwrap();
        assert_eq!(
            key.get_raw_value(&executable).unwrap_err().kind(),
            std::io::ErrorKind::NotFound,
            "Use a fresh disposable executable"
        );
        struct Guard<'a>(&'a Path, &'a str);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                for _ in 0..2 {
                    if let Err(e) = restore(self.0, self.1) {
                        if e.code != "NO_BACKUP" {
                            eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                        }
                        break;
                    }
                }
            }
        }
        let _guard = Guard(&root, &executable);
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "high".into(),
            defender: false,
        };
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        apply(&root, &executable, &profile, false).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "applied");
        let empty_profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: false,
        };
        apply(&root, &executable, &empty_profile, true).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        assert_eq!(
            key.get_raw_value(&executable).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        assert_eq!(restore(&root, &executable).unwrap_err().code, "NO_BACKUP");
        apply(&root, &executable, &profile, false).unwrap();
        restore(&root, &executable).unwrap();
        assert_eq!(
            key.get_raw_value(&executable).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        for commit in [false, true] {
            apply(&root, &executable, &profile, false).unwrap();
            let dir = resource_dir(&root, &executable).unwrap();
            let file = find_active(&dir, &executable).unwrap().unwrap();
            let mut tx = Transaction::open(&file).unwrap();
            tx.begin_reset(&requests(&executable, &profile, true).unwrap(), true)
                .unwrap();
            let previous = tx
                .journal
                .reset
                .as_ref()
                .unwrap()
                .previous_steps
                .clone()
                .unwrap();
            tx.apply(&mut ResetExecutor { previous }).unwrap();
            if commit {
                tx.start_reset_commit().unwrap();
            }
            drop(tx);
            assert_eq!(
                status(&root, &executable).unwrap().state,
                if commit { "resetCommit" } else { "recovery" }
            );
            restore(&root, &executable).unwrap();
            if !commit {
                assert_eq!(status(&root, &executable).unwrap().state, "applied");
                restore(&root, &executable).unwrap();
            }
            assert_eq!(status(&root, &executable).unwrap().state, "none");
            assert_eq!(
                key.get_raw_value(&executable).unwrap_err().kind(),
                std::io::ErrorKind::NotFound
            );
        }
    }

    #[test]
    #[ignore = "requires UAC and changes CPU, GPU and Defender settings for a disposable executable"]
    fn real_full_profile_applies_and_restores() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_SERVICE_ROOT").expect("Set GAME_OPTIMIZER_SERVICE_ROOT"),
        );
        let name = crate::privileged::cpu_name(&executable).unwrap();
        let folder = Path::new(&executable).parent().unwrap().to_str().unwrap();
        let before_cpu = crate::privileged::read_cpu(&name).unwrap();
        let before_folder = crate::privileged::query_defender("defender-path", folder).unwrap();
        let before_process =
            crate::privileged::query_defender("defender-process", &executable).unwrap();
        assert!(
            !before_folder && !before_process,
            "Use a fresh disposable executable and folder"
        );
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        struct Guard<'a>(&'a Path, &'a str);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Err(e) = restore(self.0, self.1) {
                    if e.code != "NO_BACKUP" {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        let _guard = Guard(&root, &executable);
        let profile = ProfileActions {
            cpu: "high".into(),
            gpu: "high".into(),
            defender: true,
        };
        apply(&root, &executable, &profile, false).unwrap();
        assert_eq!(
            status(&root, &executable).unwrap().state,
            if unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() } != 0 {
                "applied"
            } else {
                "unverified"
            }
        );
        assert!(crate::privileged::query_defender("defender-path", folder).unwrap());
        assert!(crate::privileged::query_defender("defender-process", &executable).unwrap());
        restore(&root, &executable).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        assert_eq!(crate::privileged::read_cpu(&name).unwrap(), before_cpu);
        assert_eq!(
            crate::privileged::query_defender("defender-path", folder).unwrap(),
            before_folder
        );
        assert_eq!(
            crate::privileged::query_defender("defender-process", &executable).unwrap(),
            before_process
        );
    }

    #[test]
    #[ignore = "requires UAC and temporarily changes Defender exclusions for a disposable executable"]
    fn real_dummy_coordinated_defender_profile_applies_and_restores() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_SERVICE_ROOT").expect("Set GAME_OPTIMIZER_SERVICE_ROOT"),
        );
        let folder = Path::new(&executable).parent().unwrap().to_str().unwrap();
        assert!(!crate::privileged::query_defender("defender-path", folder).unwrap());
        assert!(!crate::privileged::query_defender("defender-process", &executable).unwrap());
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        struct Guard<'a>(&'a Path, &'a str);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Err(e) = restore(self.0, self.1) {
                    if e.code != "NO_BACKUP" {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        let _guard = Guard(&root, &executable);
        let profile = ProfileActions {
            cpu: "none".into(),
            gpu: "none".into(),
            defender: true,
        };
        apply(&root, &executable, &profile, false).unwrap();
        assert_eq!(
            status(&root, &executable).unwrap().state,
            if unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() } != 0 {
                "applied"
            } else {
                "unverified"
            }
        );
        assert!(crate::privileged::query_defender("defender-path", folder).unwrap());
        assert!(crate::privileged::query_defender("defender-process", &executable).unwrap());
        let nested_folder = Path::new(folder).join("nested");
        fs::create_dir(&nested_folder).unwrap();
        let nested = nested_folder.join("second.exe");
        fs::copy(&executable, &nested).unwrap();
        let nested = nested.to_str().unwrap();
        let _nested_guard = Guard(&root, nested);
        let conflict = apply(&root, nested, &profile, false).unwrap_err();
        assert!(
            conflict.message.contains("overlaps"),
            "{}",
            conflict.message
        );
        assert_eq!(status(&root, nested).unwrap().state, "none");
        assert!(crate::privileged::query_defender("defender-path", folder).unwrap());
        assert!(crate::privileged::query_defender("defender-process", &executable).unwrap());
        restore(&root, &executable).unwrap();
        assert_eq!(status(&root, &executable).unwrap().state, "none");
        assert!(!crate::privileged::query_defender("defender-path", folder).unwrap());
        assert!(!crate::privileged::query_defender("defender-process", &executable).unwrap());
    }

    #[test]
    #[ignore = "requires UAC and restores a real full-profile test journal"]
    fn recover_full_profile() {
        let executable =
            std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_SERVICE_ROOT").expect("Set GAME_OPTIMIZER_SERVICE_ROOT"),
        );
        restore(&root, &executable).unwrap();
    }
}
