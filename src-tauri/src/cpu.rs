use crate::{
    gpu::{RawValue, raw, reg_type},
    machine_resource,
    privileged::{self, Job},
    transaction::{Step, StepExecutor},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;
use winreg::types::ToRegValue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    kind: String,
    path: String,
    priority: u32,
    owner: Uuid,
    #[serde(default)]
    retain_reservation_on_restore: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    value: Option<RawValue>,
}

fn action(step: &Step) -> Result<Action, String> {
    log::info!("Validating CPU action");
    let action: Action = serde_json::from_value(step.action.clone()).map_err(|e| e.to_string())?;
    privileged::cpu_name(&action.path)?;
    if action.kind != "cpu" || !matches!(action.priority, 0 | 1 | 3) {
        return Err("Invalid CPU action".into());
    }
    Ok(action)
}

fn desired(action: &Action) -> Option<RawValue> {
    if action.priority == 0 {
        None
    } else {
        Some(raw(action.priority.to_reg_value()))
    }
}

pub fn validate(step: &Step, path: &str) -> Result<(), String> {
    let action = action(step)?;
    if !action.path.eq_ignore_ascii_case(path) {
        return Err("CPU journal path mismatch".into());
    }
    Ok(())
}

pub fn snapshot_matches_applied(step: &Step, snapshot: &Value) -> Result<bool, String> {
    log::info!("Checking CPU reset snapshot against the active profile");
    let before: Snapshot = serde_json::from_value(snapshot.clone()).map_err(|e| e.to_string())?;
    Ok(before.value == desired(&action(step)?))
}

trait Cpu {
    fn read(&mut self, name: &str) -> Result<Option<RawValue>, String>;
    fn write(
        &mut self,
        name: &str,
        owner: Uuid,
        expected: Option<RawValue>,
        desired: Option<RawValue>,
    ) -> Result<(), String>;
}
struct WindowsCpu;
impl Cpu for WindowsCpu {
    fn read(&mut self, name: &str) -> Result<Option<RawValue>, String> {
        privileged::read_cpu(name)
    }
    fn write(
        &mut self,
        name: &str,
        owner: Uuid,
        expected: Option<RawValue>,
        desired: Option<RawValue>,
    ) -> Result<(), String> {
        privileged::launch(&Job::Cpu {
            name: name.into(),
            owner,
            expected,
            desired,
        })
    }
}

pub struct CpuExecutor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuPriority {
    Low,
    High,
}

impl CpuExecutor {
    fn executor(step: &Step) -> Result<Executor<WindowsCpu>, String> {
        let action = action(step)?;
        let name = privileged::cpu_name(&action.path)?;
        let (reservation, lock) = machine_resource::paths("cpu", &name)?;
        Ok(Executor {
            cpu: WindowsCpu,
            reservation,
            lock,
        })
    }
    pub fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        Self::executor(step)?.matches_applied(step)
    }
    pub fn launch_priority(&mut self, step: &Step) -> Result<Option<CpuPriority>, String> {
        let priority = match action(step)?.priority {
            0 => return Ok(None),
            1 => CpuPriority::Low,
            3 => CpuPriority::High,
            _ => return Err("Invalid CPU priority".into()),
        };
        if !self.matches_applied(step)? {
            return Err("CPU priority or reservation changed externally".into());
        }
        Ok(Some(priority))
    }
    pub fn finish_reset(&mut self, step: &Step) -> Result<(), String> {
        log::info!("Releasing CPU reservation after reset");
        Self::executor(step)?.finish_reset(step)
    }
}
impl StepExecutor for CpuExecutor {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        Self::executor(step)?.snapshot(step)
    }
    fn apply(&mut self, step: &Step) -> Result<(), String> {
        Self::executor(step)?.apply(step)
    }
    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        Self::executor(step)?.restore(step, before)
    }
}

struct Executor<P> {
    cpu: P,
    reservation: PathBuf,
    lock: PathBuf,
}
impl<P: Cpu> Executor<P> {
    fn finish_reset(&mut self, step: &Step) -> Result<(), String> {
        let action = action(step)?;
        let _guard = machine_resource::lock(&self.lock)?;
        let owner = machine_resource::owner(&self.reservation)?;
        match owner.as_deref() {
            None => Ok(()),
            Some(owner) if owner == action.owner.to_string() => {
                machine_resource::release(&self.reservation, owner)
            }
            Some(_) => Err("CPU resource reservation changed during reset".into()),
        }
    }
    fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        let action = action(step)?;
        let name = privileged::cpu_name(&action.path)?;
        let _guard = machine_resource::lock(&self.lock)?;
        Ok(machine_resource::owner(&self.reservation)?.as_deref()
            == Some(action.owner.to_string().as_str())
            && self.cpu.read(&name)? == desired(&action))
    }
}
impl<P: Cpu> StepExecutor for Executor<P> {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        log::info!("Capturing CPU priority");
        let action = action(step)?;
        let name = privileged::cpu_name(&action.path)?;
        let _guard = machine_resource::lock(&self.lock)?;
        let value = self.cpu.read(&name)?;
        if let Some(value) = &value {
            reg_type(value.kind)?;
        }
        serde_json::to_value(Snapshot { value }).map_err(|e| e.to_string())
    }

    fn apply(&mut self, step: &Step) -> Result<(), String> {
        log::info!("Applying CPU priority");
        let action = action(step)?;
        let name = privileged::cpu_name(&action.path)?;
        let before: Snapshot =
            serde_json::from_value(step.before.clone().ok_or("Missing CPU snapshot")?)
                .map_err(|e| e.to_string())?;
        let desired = desired(&action);
        {
            let _guard = machine_resource::lock(&self.lock)?;
            if action.retain_reservation_on_restore {
                if machine_resource::owner(&self.reservation)?.as_deref()
                    != Some(action.owner.to_string().as_str())
                {
                    return Err("CPU resource reservation changed before reset".into());
                }
            } else {
                machine_resource::claim(&self.reservation, &action.owner.to_string())?;
            }
            if self.cpu.read(&name)? != before.value {
                return Err("CPU priority changed since snapshot".into());
            }
        }
        if before.value != desired {
            self.cpu
                .write(&name, action.owner, before.value, desired.clone())?;
        }
        let _guard = machine_resource::lock(&self.lock)?;
        if machine_resource::owner(&self.reservation)?.as_deref()
            != Some(action.owner.to_string().as_str())
            || self.cpu.read(&name)? != desired
        {
            return Err("CPU priority verification failed".into());
        }
        Ok(())
    }

    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        log::info!("Restoring CPU priority");
        let action = action(step)?;
        let name = privileged::cpu_name(&action.path)?;
        let before: Snapshot = serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
        let expected = action.owner.to_string();
        let desired = desired(&action);
        {
            let _guard = machine_resource::lock(&self.lock)?;
            let current = self.cpu.read(&name)?;
            let owner = machine_resource::owner(&self.reservation)?;
            if action.retain_reservation_on_restore && owner.as_deref() != Some(&expected) {
                return Err("CPU resource reservation changed during reset recovery".into());
            }
            if current == before.value {
                if !action.retain_reservation_on_restore && owner.as_deref() == Some(&expected) {
                    machine_resource::release(&self.reservation, &expected)?;
                }
                return Ok(());
            }
            if owner.as_deref() != Some(&expected) || current != desired {
                if owner.as_deref() == Some(&expected) && !action.retain_reservation_on_restore {
                    let _ = machine_resource::release(&self.reservation, &expected);
                }
                return Err(
                    "CPU priority or reservation changed externally; refusing to overwrite it"
                        .into(),
                );
            }
        }
        if let Err(e) = self
            .cpu
            .write(&name, action.owner, desired, before.value.clone())
        {
            if e.contains(crate::transaction::EXTERNAL_CHANGE)
                && !action.retain_reservation_on_restore
            {
                let _ = machine_resource::release(&self.reservation, &expected);
            }
            return Err(e);
        }
        let _guard = machine_resource::lock(&self.lock)?;
        if self.cpu.read(&name)? != before.value {
            return Err("CPU priority verification failed after restoration".into());
        }
        if action.retain_reservation_on_restore {
            if machine_resource::owner(&self.reservation)?.as_deref() != Some(&expected) {
                return Err("CPU resource reservation changed during reset recovery".into());
            }
            Ok(())
        } else {
            machine_resource::release(&self.reservation, &expected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{StepRequest, Transaction};
    use serde_json::json;
    struct Fake(Option<RawValue>);
    impl Cpu for Fake {
        fn read(&mut self, _: &str) -> Result<Option<RawValue>, String> {
            Ok(self.0.clone())
        }
        fn write(
            &mut self,
            _: &str,
            _: Uuid,
            expected: Option<RawValue>,
            desired: Option<RawValue>,
        ) -> Result<(), String> {
            if self.0 != expected {
                return Err("conflict".into());
            }
            self.0 = desired;
            Ok(())
        }
    }
    #[test]
    fn cpu_transaction_preserves_prior_value_and_refuses_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let request = StepRequest {
            id: "cpu".into(),
            action: json!({"kind":"cpu","path":r"C:\Games\same.exe","priority":3,"owner":Uuid::new_v4()}),
        };
        let mut tx = Transaction::create(&dir.path().join("tx.json"), &[request]).unwrap();
        let original = Some(raw(5u32.to_reg_value()));
        let mut exec = Executor {
            cpu: Fake(original.clone()),
            reservation: dir.path().join("cpu"),
            lock: dir.path().join("cpu.lock"),
        };
        tx.apply(&mut exec).unwrap();
        assert!(exec.matches_applied(&tx.journal.steps[0]).unwrap());
        exec.cpu.0 = Some(raw(2u32.to_reg_value()));
        tx.restore(&mut exec).unwrap();
        assert_eq!(tx.skipped.len(), 1);
        assert_eq!(exec.cpu.0, Some(raw(2u32.to_reg_value())));
    }

    #[test]
    fn cpu_transaction_restores_the_prior_value() {
        let dir = tempfile::tempdir().unwrap();
        let request = StepRequest {
            id: "cpu".into(),
            action: json!({"kind":"cpu","path":r"C:\Games\same.exe","priority":3,"owner":Uuid::new_v4()}),
        };
        let mut tx = Transaction::create(&dir.path().join("tx.json"), &[request]).unwrap();
        let original = Some(raw(5u32.to_reg_value()));
        let mut exec = Executor {
            cpu: Fake(original.clone()),
            reservation: dir.path().join("cpu"),
            lock: dir.path().join("cpu.lock"),
        };
        tx.apply(&mut exec).unwrap();
        tx.restore(&mut exec).unwrap();
        assert_eq!(exec.cpu.0, original);
        assert_eq!(machine_resource::owner(&exec.reservation).unwrap(), None);
    }

    #[test]
    fn defaults_remove_priority_and_restore_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let request = StepRequest {
            id: "cpu".into(),
            action: json!({"kind":"cpu","path":r"C:\Games\same.exe","priority":0,"owner":Uuid::new_v4()}),
        };
        let mut tx = Transaction::create(&dir.path().join("tx.json"), &[request]).unwrap();
        let original = Some(raw(5u32.to_reg_value()));
        let mut exec = Executor {
            cpu: Fake(original.clone()),
            reservation: dir.path().join("cpu"),
            lock: dir.path().join("cpu.lock"),
        };
        tx.apply(&mut exec).unwrap();
        assert_eq!(exec.cpu.0, None);
        tx.restore(&mut exec).unwrap();
        assert_eq!(exec.cpu.0, original);
    }

    #[test]
    fn reset_recovery_keeps_active_reservation_and_commit_releases_it() {
        let dir = tempfile::tempdir().unwrap();
        let owner = Uuid::new_v4();
        let path = r"C:\Games\same.exe";
        let original = Some(raw(5u32.to_reg_value()));
        let profile = StepRequest {
            id: "cpu".into(),
            action: json!({"kind":"cpu","path":path,"priority":3,"owner":owner}),
        };
        let reset = StepRequest {
            id: "cpu".into(),
            action: json!({"kind":"cpu","path":path,"priority":0,"owner":owner,"retain_reservation_on_restore":true}),
        };
        let journal = dir.path().join("tx.json");
        let mut tx = Transaction::create(&journal, &[profile]).unwrap();
        let mut exec = Executor {
            cpu: Fake(original.clone()),
            reservation: dir.path().join("cpu"),
            lock: dir.path().join("cpu.lock"),
        };
        tx.apply(&mut exec).unwrap();
        let captured = exec.snapshot(&tx.journal.steps[0]).unwrap();
        assert!(snapshot_matches_applied(&tx.journal.steps[0], &captured).unwrap());
        assert!(
            !snapshot_matches_applied(&tx.journal.steps[0], &json!({"value": original})).unwrap()
        );
        tx.begin_reset(&[reset.clone()], true).unwrap();
        tx.apply(&mut exec).unwrap();
        drop(tx);
        let mut tx = Transaction::open(&journal).unwrap();
        tx.rollback_reset(&mut exec).unwrap();
        assert_eq!(exec.cpu.0, Some(raw(3u32.to_reg_value())));
        assert_eq!(
            machine_resource::owner(&exec.reservation)
                .unwrap()
                .as_deref(),
            Some(owner.to_string().as_str())
        );
        tx.begin_reset(&[reset], true).unwrap();
        tx.apply(&mut exec).unwrap();
        tx.start_reset_commit().unwrap();
        exec.finish_reset(&tx.journal.steps[0]).unwrap();
        exec.finish_reset(&tx.journal.steps[0]).unwrap();
        tx.finish_reset().unwrap();
        assert_eq!(exec.cpu.0, None);
        assert_eq!(machine_resource::owner(&exec.reservation).unwrap(), None);
        assert!(tx.journal.steps[0].before.is_none());
    }
}
