fn main() {
    let cwd = std::env::current_dir().unwrap();
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn IsUserAnAdmin() -> i32;
    }
    std::fs::write(
        cwd.join("launch-marker.txt"),
        format!("{}\n{}", cwd.to_string_lossy(), unsafe { IsUserAnAdmin() }),
    )
    .unwrap();
}
