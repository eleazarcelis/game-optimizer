use crate::resource_identity::Identity;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use windows_sys::Win32::{
    System::Com::CoTaskMemFree,
    UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath},
};

pub fn program_data() -> Result<PathBuf, String> {
    let mut buffer = std::ptr::null_mut();
    // The returned path is allocated by Windows and must be released with CoTaskMemFree.
    let result = unsafe {
        SHGetKnownFolderPath(&FOLDERID_ProgramData, 0, std::ptr::null_mut(), &mut buffer)
    };
    if result < 0 {
        return Err(format!("Cannot locate ProgramData: HRESULT {result:#x}"));
    }
    if buffer.is_null() {
        return Err("ProgramData path is null".into());
    }
    let mut length = 0;
    unsafe {
        while *buffer.add(length) != 0 {
            length += 1;
        }
        let path = String::from_utf16(std::slice::from_raw_parts(buffer, length));
        CoTaskMemFree(buffer.cast());
        let path = path.map_err(|e| format!("Invalid ProgramData path: {e}"))?;
        Ok(PathBuf::from(path))
    }
}

pub fn directory() -> Result<PathBuf, String> {
    log::info!("Resolving machine-wide resource directory");
    let directory = program_data()?.join("GameOptimizer").join("resources");
    fs::create_dir_all(&directory).map_err(|e| format!("Cannot create resource directory: {e}"))?;
    Ok(directory)
}

pub fn claim(path: &Path, owner: &str) -> Result<(), String> {
    log::info!("Claiming machine-wide resource");
    let dir = path.parent().ok_or("Resource path has no parent")?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("Cannot create reservation: {e}"))?;
    temp.write_all(owner.as_bytes())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| format!("Cannot persist reservation: {e}"))?;
    temp.persist_noclobber(path).map_err(|e| {
        format!(
            "Resource already reserved or cannot be claimed: {}",
            e.error
        )
    })?;
    Ok(())
}

pub fn owner(path: &Path) -> Result<Option<String>, String> {
    log::info!("Checking machine-wide resource owner");
    match fs::read_to_string(path) {
        Ok(value) if !value.is_empty() && value.len() <= 128 => Ok(Some(value)),
        Ok(_) => Err("Invalid resource reservation".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read resource reservation: {e}")),
    }
}

pub fn release(path: &Path, expected: &str) -> Result<(), String> {
    log::info!("Releasing machine-wide resource");
    if owner(path)?.as_deref() != Some(expected) {
        return Err("Resource reservation belongs to another transaction or is missing".into());
    }
    fs::remove_file(path).map_err(|e| format!("Cannot release resource reservation: {e}"))
}

pub fn paths(kind: &str, value: &str) -> Result<(PathBuf, PathBuf), String> {
    log::info!("Resolving machine-wide action resource");
    if !matches!(kind, "cpu" | "defender-path" | "defender-process") {
        return Err("Invalid resource kind".into());
    }
    let normalized = value.replace('/', "\\").to_lowercase();
    let name = format!("{kind}-{:x}", Sha256::digest(normalized.as_bytes()));
    let dir = directory()?;
    Ok((dir.join(&name), dir.join(format!("{name}.lock"))))
}

pub fn defender_identity_paths(
    kind: &str,
    identity: &Identity,
) -> Result<(PathBuf, PathBuf), String> {
    log::info!("Resolving physical Defender resource reservation");
    let name = defender_identity_name(kind, identity)?;
    let dir = directory()?;
    Ok((dir.join(&name), dir.join(format!("{name}.lock"))))
}

fn defender_identity_name(kind: &str, identity: &Identity) -> Result<String, String> {
    if !matches!(kind, "defender-path" | "defender-process") {
        return Err("Invalid Defender resource kind".into());
    }
    let mut hash = Sha256::new();
    hash.update(identity.volume.to_le_bytes());
    hash.update(identity.file_id);
    Ok(format!("{kind}-identity-{:x}", hash.finalize()))
}

pub fn lock(path: &Path) -> Result<fs::File, String> {
    log::info!("Locking machine-wide action resource");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("Cannot open resource lock: {e}"))?;
    file.try_lock_exclusive()
        .map_err(|e| format!("Resource is busy: {e}"))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservation_is_exclusive_and_owner_checked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("power");
        claim(&path, "owner-a").unwrap();
        assert!(claim(&path, "owner-b").is_err());
        assert!(release(&path, "owner-b").is_err());
        assert_eq!(owner(&path).unwrap().as_deref(), Some("owner-a"));
        release(&path, "owner-a").unwrap();
        claim(&path, "owner-b").unwrap();
    }

    #[test]
    fn hard_link_aliases_use_the_same_resource_name() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("one.exe");
        let b = dir.path().join("two.exe");
        fs::write(&a, b"test").unwrap();
        fs::hard_link(&a, &b).unwrap();
        let a = crate::resource_identity::Target::open(a.to_str().unwrap(), false).unwrap();
        let b = crate::resource_identity::Target::open(b.to_str().unwrap(), false).unwrap();
        assert_eq!(
            defender_identity_name("defender-process", a.identity()).unwrap(),
            defender_identity_name("defender-process", b.identity()).unwrap()
        );
    }
}
