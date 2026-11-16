fn main() {
    let cwd = std::env::current_dir().unwrap();
    std::fs::write(
        cwd.join("launch-marker.txt"),
        cwd.to_string_lossy().as_bytes(),
    )
    .unwrap();
}
