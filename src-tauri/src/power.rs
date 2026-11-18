use crate::{
    machine_resource,
    transaction::{Step, StepExecutor},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;
use windows_sys::{
    Win32::{
        Foundation::LocalFree,
        System::Power::{PowerGetActiveScheme, PowerSetActiveScheme},
    },
    core::GUID,
};

pub const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
pub const HIGH: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    kind: String,
    scheme: Uuid,
    owner: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    scheme: Uuid,
}

fn action(step: &Step) -> Result<Action, String> {
    log::info!("Validating power transaction action");
    let action: Action = serde_json::from_value(step.action.clone()).map_err(|e| e.to_string())?;
    if action.kind != "power"
        || ![BALANCED, HIGH]
            .iter()
            .any(|s| Uuid::parse_str(s).ok() == Some(action.scheme))
    {
        return Err("Invalid power action".into());
    }
    Ok(action)
}

pub fn validate(step: &Step) -> Result<(), String> {
    action(step).map(|_| ())
}

trait Power {
    fn read(&mut self) -> Result<Uuid, String>;
    fn write(&mut self, scheme: Uuid) -> Result<(), String>;
}

struct WindowsPower;
impl Power for WindowsPower {
    fn read(&mut self) -> Result<Uuid, String> {
        log::info!("Reading active power plan");
        let mut pointer: *mut GUID = std::ptr::null_mut();
        let result = unsafe { PowerGetActiveScheme(std::ptr::null_mut(), &mut pointer) };
        if result != 0 {
            return Err(format!("PowerGetActiveScheme failed: {result}"));
        }
        if pointer.is_null() {
            return Err("PowerGetActiveScheme returned no plan".into());
        }
        // Windows allocates the GUID with LocalAlloc.
        let scheme = unsafe {
            let guid = *pointer;
            LocalFree(pointer.cast());
            Uuid::from_fields(guid.data1, guid.data2, guid.data3, &guid.data4)
        };
        Ok(scheme)
    }

    fn write(&mut self, scheme: Uuid) -> Result<(), String> {
        log::info!("Changing active power plan");
        let guid = GUID::from_u128(scheme.as_u128());
        let result = unsafe { PowerSetActiveScheme(std::ptr::null_mut(), &guid) };
        if result != 0 {
            return Err(format!("PowerSetActiveScheme failed: {result}"));
        }
        Ok(())
    }
}

pub struct PowerExecutor {
    inner: Executor<WindowsPower>,
}
impl PowerExecutor {
    pub fn new() -> Result<Self, String> {
        log::info!("Preparing power executor");
        Ok(Self {
            inner: Executor {
                power: WindowsPower,
                reservation: machine_resource::directory()?.join("power-plan"),
            },
        })
    }
    pub fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        self.inner.matches_applied(step)
    }
}
impl StepExecutor for PowerExecutor {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        self.inner.snapshot(step)
    }
    fn apply(&mut self, step: &Step) -> Result<(), String> {
        self.inner.apply(step)
    }
    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        self.inner.restore(step, before)
    }
}

struct Executor<P> {
    power: P,
    reservation: PathBuf,
}
impl<P: Power> Executor<P> {
    fn matches_applied(&mut self, step: &Step) -> Result<bool, String> {
        log::info!("Checking applied power plan");
        let action = action(step)?;
        Ok(machine_resource::owner(&self.reservation)?.as_deref()
            == Some(action.owner.to_string().as_str())
            && self.power.read()? == action.scheme)
    }
}
impl<P: Power> StepExecutor for Executor<P> {
    fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
        log::info!("Capturing active power plan");
        action(step)?;
        serde_json::to_value(Snapshot {
            scheme: self.power.read()?,
        })
        .map_err(|e| e.to_string())
    }

    fn apply(&mut self, step: &Step) -> Result<(), String> {
        log::info!("Applying power plan");
        let action = action(step)?;
        let before: Snapshot =
            serde_json::from_value(step.before.clone().ok_or("Missing power snapshot")?)
                .map_err(|e| e.to_string())?;
        machine_resource::claim(&self.reservation, &action.owner.to_string())?;
        if self.power.read()? != before.scheme {
            return Err("Power plan changed since the snapshot".into());
        }
        self.power.write(action.scheme)?;
        if self.power.read()? != action.scheme {
            return Err("Power plan verification failed after writing".into());
        }
        Ok(())
    }

    fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
        log::info!("Restoring power plan");
        let action = action(step)?;
        let before: Snapshot = serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
        let current = self.power.read()?;
        let owner = machine_resource::owner(&self.reservation)?;
        let expected = action.owner.to_string();
        if current == before.scheme {
            if owner.as_deref() == Some(&expected) {
                machine_resource::release(&self.reservation, &expected)?;
            }
            return Ok(());
        }
        if owner.as_deref() != Some(&expected) {
            return Err("Power reservation is missing or belongs to another transaction".into());
        }
        if current != action.scheme {
            let _ = machine_resource::release(&self.reservation, &expected);
            return Err("Power plan changed externally; refusing to overwrite it".into());
        }
        self.power.write(before.scheme)?;
        if self.power.read()? != before.scheme {
            return Err("Power plan verification failed after restoring".into());
        }
        machine_resource::release(&self.reservation, &expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{StepRequest, Transaction};
    use serde_json::json;
    struct Fake(Uuid);
    impl Power for Fake {
        fn read(&mut self) -> Result<Uuid, String> {
            Ok(self.0)
        }
        fn write(&mut self, value: Uuid) -> Result<(), String> {
            self.0 = value;
            Ok(())
        }
    }
    #[test]
    fn power_reservation_detects_conflicts_and_restores_exact_plan() {
        let dir = tempfile::tempdir().unwrap();
        let plan = Uuid::new_v4();
        let token = Uuid::new_v4();
        let request = StepRequest {
            id: "power".into(),
            action: json!({"kind":"power","scheme":HIGH,"owner":token}),
        };
        let path = dir.path().join("tx.json");
        let mut tx = Transaction::create(&path, &[request]).unwrap();
        let mut executor = Executor {
            power: Fake(plan),
            reservation: dir.path().join("power"),
        };
        tx.apply(&mut executor).unwrap();
        assert!(executor.matches_applied(&tx.journal.steps[0]).unwrap());
        assert!(machine_resource::claim(&executor.reservation, "other").is_err());
        executor.power.0 = Uuid::new_v4();
        tx.restore(&mut executor).unwrap();
        assert_eq!(tx.skipped.len(), 1);
        assert_eq!(
            machine_resource::owner(&executor.reservation).unwrap(),
            None
        );
    }

    #[test]
    #[ignore = "temporarily changes the power plan for the whole computer"]
    fn real_power_recovers_after_partial_failure() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_POWER_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_POWER_TEST_ROOT"),
        );
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("power-journal.json");
        assert!(!path.exists(), "Use a fresh persistent journal");
        let mut current = WindowsPower;
        let before = current.read().unwrap();
        let desired = if before == Uuid::parse_str(HIGH).unwrap() {
            BALANCED
        } else {
            HIGH
        };
        let request = StepRequest {
            id: "power".into(),
            action: json!({"kind":"power","scheme":desired,"owner":Uuid::new_v4()}),
        };
        struct Guard(PathBuf);
        impl Drop for Guard {
            fn drop(&mut self) {
                if self.0.exists() {
                    if let Err(e) = Transaction::open(&self.0).and_then(|mut tx| {
                        let mut executor = PowerExecutor::new().map_err(|message| {
                            crate::transaction::TransactionError {
                                code: "POWER_INIT",
                                step: None,
                                message,
                            }
                        })?;
                        tx.restore(&mut executor)
                    }) {
                        eprintln!("RECOVERY REQUIRED: {}: {}", e.code, e.message);
                    }
                }
            }
        }
        struct FailAfterWrite(PowerExecutor);
        impl StepExecutor for FailAfterWrite {
            fn snapshot(&mut self, step: &Step) -> Result<Value, String> {
                self.0.snapshot(step)
            }
            fn apply(&mut self, step: &Step) -> Result<(), String> {
                self.0.apply(step)?;
                Err("Simulated interruption after write".into())
            }
            fn restore(&mut self, step: &Step, before: &Value) -> Result<(), String> {
                self.0.restore(step, before)
            }
        }
        let _guard = Guard(path.clone());
        let mut tx = Transaction::create(&path, &[request]).unwrap();
        assert_eq!(
            tx.apply(&mut FailAfterWrite(PowerExecutor::new().unwrap()))
                .unwrap_err()
                .code,
            "APPLY_FAILED"
        );
        assert_eq!(current.read().unwrap(), Uuid::parse_str(desired).unwrap());
        drop(tx);
        Transaction::open(&path)
            .unwrap()
            .restore(&mut PowerExecutor::new().unwrap())
            .unwrap();
        assert_eq!(current.read().unwrap(), before);
    }

    #[test]
    #[ignore = "restores a real power plan test journal"]
    fn recover_power_test_journal() {
        let root = PathBuf::from(
            std::env::var("GAME_OPTIMIZER_POWER_TEST_ROOT")
                .expect("Set GAME_OPTIMIZER_POWER_TEST_ROOT"),
        );
        Transaction::open(&root.join("power-journal.json"))
            .unwrap()
            .restore(&mut PowerExecutor::new().unwrap())
            .unwrap();
    }
}
