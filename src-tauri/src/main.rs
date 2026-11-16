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
        if let Err(detail) = game_optimizer_app_lib::check_desktop() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
            let message = format!(
                "No se puede iniciar Game Optimizer. Usa una cuenta administradora en la misma sesión de Windows y asegúrate de que el escritorio no esté elevado.\n\nGame Optimizer cannot start. Use an administrator account in the same Windows session and make sure the desktop is not elevated.\n\n{detail}\0"
            );
            let text: Vec<u16> = message.encode_utf16().collect();
            let title: Vec<u16> = "Game Optimizer\0".encode_utf16().collect();
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    text.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
            std::process::exit(1);
        }
    }
    game_optimizer_app_lib::run()
}
