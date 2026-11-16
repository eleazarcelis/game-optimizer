use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo,
    GetFileInformationByHandleEx,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub(crate) volume: u64,
    pub(crate) file_id: [u8; 16],
}

pub struct Target {
    file: File,
    identity: Identity,
}

impl Target {
    pub fn open(path: &str, directory: bool) -> Result<Self, String> {
        log::info!("Resolving physical resource identity");
        let file = OpenOptions::new()
            .access_mode(0)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(Path::new(path))
            .map_err(|e| format!("Cannot open resource for identity verification: {e}"))?;
        let metadata = file
            .metadata()
            .map_err(|e| format!("Cannot inspect resource type: {e}"))?;
        if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
            return Err("Resource type does not match the Defender action".into());
        }
        let mut info = FILE_ID_INFO::default();
        let result = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle().cast(),
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        };
        if result == 0 {
            return Err(format!(
                "Cannot obtain resource identity: {}",
                std::io::Error::last_os_error()
            ));
        }
        if info.FileId.Identifier == [0; 16] {
            return Err("Filesystem returned an invalid resource identity".into());
        }
        Ok(Self {
            file,
            identity: Identity {
                volume: info.VolumeSerialNumber,
                file_id: info.FileId.Identifier,
            },
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn ensure_path(&self, path: &str, directory: bool) -> Result<(), String> {
        let current = Self::open(path, directory)?;
        if current.identity != self.identity {
            return Err("Resource identity changed; refusing to modify Defender".into());
        }
        // Keep the original handle open while the path is rechecked and the operation runs.
        let _ = &self.file;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_links_share_identity_and_replacement_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("game.exe");
        let alias = dir.path().join("alias.exe");
        std::fs::write(&original, b"one").unwrap();
        std::fs::hard_link(&original, &alias).unwrap();
        let first = Target::open(original.to_str().unwrap(), false).unwrap();
        let second = Target::open(alias.to_str().unwrap(), false).unwrap();
        assert_eq!(first.identity(), second.identity());
        assert!(Target::open(dir.path().to_str().unwrap(), false).is_err());
        assert!(Target::open(dir.path().to_str().unwrap(), true).is_ok());
        drop(first);
        drop(second);
        std::fs::remove_file(&original).unwrap();
        std::fs::write(&original, b"two").unwrap();
        let replacement = Target::open(original.to_str().unwrap(), false).unwrap();
        let still_original = Target::open(alias.to_str().unwrap(), false).unwrap();
        assert_ne!(replacement.identity(), still_original.identity());
    }
}
