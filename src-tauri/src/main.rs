// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    {
        let mut args = std::env::args();
        args.next();
        if args.next().as_deref() == Some("--admin-helper") {
            let Some(payload) = args.next() else {
                std::process::exit(2);
            };
            if args.next().is_some() {
                std::process::exit(2);
            }
            std::process::exit(game_optimizer_app_lib::run_admin_helper(&payload));
        }
    }
    game_optimizer_app_lib::run()
}
