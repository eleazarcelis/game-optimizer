#![windows_subsystem = "windows"]

use std::{fs, path::Path, process::Command, thread, time::Duration};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("child") {
        for _ in 0..200 {
            if Path::new("stop").exists() {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("child")
        .spawn()
        .unwrap();
    fs::write("pids", format!("{} {}", std::process::id(), child.id())).unwrap();
    child.wait().unwrap();
}
