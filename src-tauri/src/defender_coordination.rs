use crate::{
    machine_resource,
    resource_identity::{Identity, Target},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::Write,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    ptr,
};
use uuid::Uuid;
use windows_sys::Win32::{
    Foundation::{ERROR_ALREADY_EXISTS, LocalFree},
    Security::{
        Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, EqualSid, GetSecurityDescriptorControl,
        GetSecurityDescriptorDacl, GetSecurityDescriptorOwner, OWNER_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        CreateDirectoryW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
    },
};

const DIRECTORY: &str = "GameOptimizer-Defender-2739818f";
const SDDL: &str = "O:BAD:P(A;OICI;0x001f01ff;;;BA)(A;OICI;0x001f01ff;;;SY)";
pub const OVERLAP: &str = "Defender exclusion overlaps another active profile";
pub const PROCESS_OVERLAP: &str = "Defender process exclusion belongs to another active profile";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    Folder,
    Process,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Scopes {
    folder: bool,
    process: bool,
}

impl Scopes {
    fn contains(&self, scope: Scope) -> bool {
        match scope {
            Scope::Folder => self.folder,
            Scope::Process => self.process,
        }
    }

    fn set(&mut self, scope: Scope, value: bool) {
        match scope {
            Scope::Folder => self.folder = value,
            Scope::Process => self.process = value,
        }
    }
}

struct SecurityDescriptor(PSECURITY_DESCRIPTOR);
impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0.cast());
        }
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn descriptor(sddl: &str) -> Result<SecurityDescriptor, String> {
    let sddl = wide(OsStr::new(sddl));
    let mut raw = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut raw,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(format!(
            "Cannot build Defender security descriptor: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(SecurityDescriptor(raw))
}

fn verify_security(file: &File, expected: &SecurityDescriptor) -> Result<(), String> {
    let mut owner = ptr::null_mut();
    let mut acl = ptr::null_mut();
    let mut raw = ptr::null_mut();
    let code = unsafe {
        GetSecurityInfo(
            file.as_raw_handle().cast(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut raw,
        )
    };
    if code != 0 {
        return Err(format!(
            "Cannot inspect Defender reservation security: Windows error {code}"
        ));
    }
    let actual = SecurityDescriptor(raw);
    let mut expected_owner = ptr::null_mut();
    let mut expected_acl = ptr::null_mut();
    let mut present = 0;
    let mut defaulted = 0;
    let mut control = 0;
    let mut revision = 0;
    let valid = unsafe {
        GetSecurityDescriptorOwner(expected.0, &mut expected_owner, &mut defaulted) != 0
            && GetSecurityDescriptorDacl(
                expected.0,
                &mut present,
                &mut expected_acl,
                &mut defaulted,
            ) != 0
            && GetSecurityDescriptorControl(actual.0, &mut control, &mut revision) != 0
            && !owner.is_null()
            && !expected_owner.is_null()
            && !acl.is_null()
            && !expected_acl.is_null()
            && present != 0
            && control & SE_DACL_PROTECTED != 0
            && EqualSid(owner, expected_owner) != 0
            && (*acl).AclSize == (*expected_acl).AclSize
            && std::slice::from_raw_parts(acl.cast::<u8>(), (*acl).AclSize as usize)
                == std::slice::from_raw_parts(
                    expected_acl.cast::<u8>(),
                    (*expected_acl).AclSize as usize,
                )
    };
    if !valid {
        return Err("Defender reservation directory has unsafe permissions or ownership".into());
    }
    Ok(())
}

pub struct Coordinator {
    directory: PathBuf,
    _directory_handle: File,
    _lock: File,
}

impl Coordinator {
    #[cfg(test)]
    pub(crate) fn for_test(directory: &Path) -> Self {
        Self {
            directory: directory.to_owned(),
            _directory_handle: OpenOptions::new()
                .access_mode(0)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                .open(directory)
                .unwrap(),
            _lock: machine_resource::lock(&directory.join("coordination.lock")).unwrap(),
        }
    }

    pub fn open() -> Result<Self, String> {
        let directory = machine_resource::program_data()?.join(DIRECTORY);
        let security = descriptor(SDDL)?;
        let handle = open_directory(&directory, &security)?;
        let lock = machine_resource::lock(&directory.join("coordination.lock"))?;
        Ok(Self {
            directory,
            _directory_handle: handle,
            _lock: lock,
        })
    }

    fn path(&self, owner: Uuid) -> PathBuf {
        self.directory.join(format!("{owner}.json"))
    }

    fn records(&self) -> Result<Vec<Reservation>, String> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.directory)
            .map_err(|e| format!("Cannot read Defender reservations: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Cannot read Defender reservation: {e}"))?;
            let path = entry.path();
            if path.extension().is_none_or(|value| value != "json") {
                continue;
            }
            let bytes =
                fs::read(&path).map_err(|e| format!("Cannot read Defender reservation: {e}"))?;
            let record: Reservation = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Invalid Defender reservation: {e}"))?;
            if record.ancestors.is_empty()
                || path != self.path(record.owner)
                || record
                    .scopes
                    .as_ref()
                    .is_some_and(|s| !s.folder && !s.process)
            {
                return Err("Invalid Defender reservation identity".into());
            }
            records.push(record);
        }
        Ok(records)
    }

    pub fn claim(&self, requested: &Reservation, scope: Scope) -> Result<(), String> {
        let records = self.records()?;
        let mut existing = None;
        for record in &records {
            if record.owner == requested.owner {
                if !record.same_identity(requested) {
                    return Err("Defender reservation identity changed".into());
                }
                if record.has_scope(scope) {
                    return Ok(());
                }
                existing = Some(record.clone());
                continue;
            }
            if record.has_scope(scope) && record.conflicts(requested, scope) {
                return Err(match scope {
                    Scope::Folder => OVERLAP,
                    Scope::Process => PROCESS_OVERLAP,
                }
                .into());
            }
        }
        let mut updated = existing.clone().unwrap_or_else(|| {
            let mut value = requested.clone();
            value.scopes = Some(Scopes {
                folder: false,
                process: false,
            });
            value
        });
        updated
            .scopes
            .as_mut()
            .expect("legacy reservations already own both scopes")
            .set(scope, true);
        self.write(&updated, existing.is_some())
    }

    fn write(&self, record: &Reservation, replace: bool) -> Result<(), String> {
        let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(|e| format!("Cannot create Defender reservation: {e}"))?;
        temp.write_all(&bytes)
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|e| format!("Cannot persist Defender reservation: {e}"))?;
        if replace {
            temp.persist(self.path(record.owner))
                .map_err(|e| format!("Cannot update Defender reservation: {}", e.error))?;
        } else {
            temp.persist_noclobber(self.path(record.owner))
                .map_err(|e| format!("Cannot claim Defender reservation: {}", e.error))?;
        }
        Ok(())
    }

    pub fn owned(&self, requested: &Reservation) -> Result<bool, String> {
        for record in self.records()? {
            if record.owner == requested.owner {
                return if record.same_identity(requested) {
                    Ok(true)
                } else {
                    Err("Defender reservation identity changed".into())
                };
            }
        }
        Ok(false)
    }

    pub fn owns_scope(&self, requested: &Reservation, scope: Scope) -> Result<bool, String> {
        for record in self.records()? {
            if record.owner == requested.owner {
                if !record.same_identity(requested) {
                    return Err("Defender reservation identity changed".into());
                }
                return Ok(record.has_scope(scope));
            }
        }
        Ok(false)
    }

    pub fn folder_conflicts(
        &self,
        requested: &Reservation,
    ) -> Result<Vec<(Uuid, Option<String>)>, String> {
        let mut conflicts = Vec::new();
        for record in self.records()? {
            if record.owner == requested.owner
                || !record.has_scope(Scope::Folder)
                || !record.conflicts(requested, Scope::Folder)
            {
                continue;
            }
            let path = match record.folder_path {
                Some(path) => match Target::open(&path, true) {
                    Ok(target) if target.identity() == &record.ancestors[0] => Some(path),
                    Ok(_) => {
                        log::warn!("Conflicting Defender folder identity changed");
                        None
                    }
                    Err(error) => {
                        log::warn!("Cannot identify conflicting Defender folder: {error}");
                        None
                    }
                },
                None => None,
            };
            conflicts.push((record.owner, path));
        }
        Ok(conflicts)
    }

    pub fn release_scope(&self, requested: &Reservation, scope: Scope) -> Result<(), String> {
        for mut record in self.records()? {
            if record.owner != requested.owner {
                continue;
            }
            if !record.same_identity(requested) || !record.has_scope(scope) {
                return Err("Defender reservation identity changed".into());
            }
            if record.scopes.is_none() {
                return if scope == Scope::Folder {
                    self.release(requested)
                } else {
                    Ok(())
                };
            }
            let scopes = record.scopes.as_mut().expect("checked above");
            scopes.set(scope, false);
            return if scopes.folder || scopes.process {
                self.write(&record, true)
            } else {
                self.release(requested)
            };
        }
        Err("Defender reservation is missing".into())
    }

    pub fn release(&self, requested: &Reservation) -> Result<(), String> {
        if !self.owned(requested)? {
            return Err("Defender reservation is missing".into());
        }
        fs::remove_file(self.path(requested.owner))
            .map_err(|e| format!("Cannot release Defender reservation: {e}"))
    }
}

pub fn ensure_no_legacy_reservations() -> Result<(), String> {
    let legacy = machine_resource::directory()?;
    for entry in fs::read_dir(legacy)
        .map_err(|e| format!("Cannot inspect previous Defender reservations: {e}"))?
    {
        let entry =
            entry.map_err(|e| format!("Cannot inspect previous Defender reservation: {e}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("defender-") && !name.ends_with(".lock") {
            return Err(
                "Restore previous Defender transactions before applying a new exclusion".into(),
            );
        }
    }
    Ok(())
}

fn open_directory(directory: &Path, security: &SecurityDescriptor) -> Result<File, String> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security.0,
        bInheritHandle: 0,
    };
    if unsafe { CreateDirectoryW(wide(directory.as_os_str()).as_ptr(), &attributes) } == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32) {
            return Err(format!(
                "Cannot create Defender reservation directory: {error}"
            ));
        }
    }
    let handle = OpenOptions::new()
        .access_mode(0x00020080)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&directory)
        .map_err(|e| format!("Cannot open Defender reservation directory: {e}"))?;
    let metadata = handle
        .metadata()
        .map_err(|e| format!("Cannot inspect Defender reservation directory: {e}"))?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err("Invalid Defender reservation directory".into());
    }
    verify_security(&handle, security)?;
    Ok(handle)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reservation {
    owner: Uuid,
    ancestors: Vec<Identity>,
    executable: Identity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scopes: Option<Scopes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    folder_path: Option<String>,
}

impl Reservation {
    pub fn open(path: &str, owner: Uuid) -> Result<(Self, Vec<Target>), String> {
        let path = Path::new(path);
        let folder = path.parent().ok_or("Missing executable folder")?;
        let canonical = fs::canonicalize(folder)
            .map_err(|e| format!("Cannot resolve executable folder: {e}"))?;
        let mut held = Vec::new();
        let mut ancestors = Vec::new();
        for root in [canonical.as_path(), folder] {
            for ancestor in root.ancestors() {
                let path = ancestor.to_str().ok_or("Invalid executable folder path")?;
                let target = Target::open(path, true)?;
                if !ancestors.contains(target.identity()) {
                    ancestors.push(target.identity().clone());
                }
                held.push(target);
            }
        }
        let target = Target::open(path.to_str().ok_or("Invalid executable path")?, false)?;
        let executable = target.identity().clone();
        held.push(target);
        Ok((
            Self {
                owner,
                ancestors,
                executable,
                scopes: None,
                folder_path: Some(folder.to_string_lossy().into_owned()),
            },
            held,
        ))
    }

    fn same_identity(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.ancestors == other.ancestors
            && self.executable == other.executable
    }

    fn has_scope(&self, scope: Scope) -> bool {
        self.scopes
            .as_ref()
            .is_none_or(|scopes| scopes.contains(scope))
    }

    fn conflicts(&self, other: &Self, scope: Scope) -> bool {
        match scope {
            Scope::Process => self.executable == other.executable,
            Scope::Folder => {
                self.ancestors.contains(&other.ancestors[0])
                    || other.ancestors.contains(&self.ancestors[0])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{
            Authorization::ConvertSidToStringSidW, GetTokenInformation, TOKEN_QUERY, TOKEN_USER,
            TokenUser,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    fn current_sid() -> String {
        let mut token = ptr::null_mut();
        assert_ne!(
            unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) },
            0
        );
        let mut size = 0;
        unsafe {
            GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut size);
        }
        let mut buffer = vec![0u64; (size as usize + 7) / 8];
        assert_ne!(
            unsafe {
                GetTokenInformation(
                    token,
                    TokenUser,
                    buffer.as_mut_ptr().cast(),
                    size,
                    &mut size,
                )
            },
            0
        );
        unsafe {
            CloseHandle(token);
        }
        let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        let mut sid = ptr::null_mut();
        assert_ne!(
            unsafe { ConvertSidToStringSidW(user.User.Sid, &mut sid) },
            0
        );
        let mut len = 0;
        unsafe {
            while *sid.add(len) != 0 {
                len += 1;
            }
        }
        let value = String::from_utf16(unsafe { std::slice::from_raw_parts(sid, len) }).unwrap();
        unsafe {
            LocalFree(sid.cast());
        }
        value
    }

    #[test]
    fn validates_directory_owner_and_permissions() {
        let root = tempfile::tempdir().unwrap();
        let sid = current_sid();
        let expected = descriptor(&format!(
            "O:{sid}D:P(A;OICI;0x001f01ff;;;{sid})(A;OICI;0x001f01ff;;;SY)"
        ))
        .unwrap();
        let dir = root.path().join("protected");
        let handle = open_directory(&dir, &expected).unwrap();
        verify_security(&handle, &expected).unwrap();
        let unsafe_acl = descriptor(&format!(
            "O:{sid}D:P(A;OICI;0x001f01ff;;;{sid})(A;OICI;0x001f01ff;;;SY)(A;OICI;0x00120089;;;WD)"
        ))
        .unwrap();
        assert!(verify_security(&handle, &unsafe_acl).is_err());
        drop(handle);
    }

    fn identity(id: u8) -> Identity {
        Identity {
            volume: 1,
            file_id: [id; 16],
        }
    }
    fn reservation(owner: Uuid, ancestors: &[u8], executable: u8) -> Reservation {
        Reservation {
            owner,
            ancestors: ancestors.iter().map(|id| identity(*id)).collect(),
            executable: identity(executable),
            scopes: None,
            folder_path: None,
        }
    }

    #[test]
    fn detects_equal_nested_and_hard_linked_resources_but_not_siblings() {
        let root = reservation(Uuid::new_v4(), &[3, 2, 1], 10);
        assert!(root.conflicts(&reservation(Uuid::new_v4(), &[3, 2, 1], 11), Scope::Folder));
        assert!(root.conflicts(
            &reservation(Uuid::new_v4(), &[4, 3, 2, 1], 11),
            Scope::Folder
        ));
        assert!(root.conflicts(&reservation(Uuid::new_v4(), &[2, 1], 11), Scope::Folder));
        assert!(root.conflicts(&reservation(Uuid::new_v4(), &[5, 2, 1], 10), Scope::Process));
        assert!(!root.conflicts(&reservation(Uuid::new_v4(), &[5, 2, 1], 11), Scope::Folder));
    }

    #[test]
    fn resolves_physical_ancestry_and_hard_links() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let child = first.join("child");
        let sibling = root.path().join("sibling");
        fs::create_dir_all(&child).unwrap();
        fs::create_dir_all(&sibling).unwrap();
        let a = first.join("a.exe");
        let b = child.join("b.exe");
        let c = sibling.join("c.exe");
        fs::write(&a, b"a").unwrap();
        fs::write(&b, b"b").unwrap();
        fs::hard_link(&a, &c).unwrap();
        let (a, a_handles) = Reservation::open(a.to_str().unwrap(), Uuid::new_v4()).unwrap();
        let (b, b_handles) = Reservation::open(b.to_str().unwrap(), Uuid::new_v4()).unwrap();
        let (c, c_handles) = Reservation::open(c.to_str().unwrap(), Uuid::new_v4()).unwrap();
        assert!(a.conflicts(&b, Scope::Folder));
        assert!(a.conflicts(&c, Scope::Process));
        assert!(!b.conflicts(&c, Scope::Folder));
        drop((a_handles, b_handles, c_handles));
    }

    #[test]
    fn detects_logical_overlap_through_a_directory_junction() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let outside = root.path().join("outside");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&outside).unwrap();
        let junction = parent.join("linked");
        let output = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "New-Item -ItemType Junction -Path $env:GO_TEST_LINK -Target $env:GO_TEST_TARGET -ErrorAction Stop | Out-Null",
            ])
            .env("GO_TEST_LINK", &junction)
            .env("GO_TEST_TARGET", &outside)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let parent_exe = parent.join("parent.exe");
        fs::write(&parent_exe, b"one").unwrap();
        fs::write(outside.join("child.exe"), b"two").unwrap();
        let (parent, parent_handles) =
            Reservation::open(parent_exe.to_str().unwrap(), Uuid::new_v4()).unwrap();
        let (child, child_handles) =
            Reservation::open(junction.join("child.exe").to_str().unwrap(), Uuid::new_v4())
                .unwrap();
        assert!(parent.conflicts(&child, Scope::Folder));
        drop((parent_handles, child_handles));
        fs::remove_dir(&junction).unwrap();
    }

    #[test]
    fn reservation_is_durable_and_rejects_overlaps() {
        let root = tempfile::tempdir().unwrap();
        let handle = File::create(root.path().join("handle")).unwrap();
        let lock = machine_resource::lock(&root.path().join("coordination.lock")).unwrap();
        let coordinator = Coordinator {
            directory: root.path().to_owned(),
            _directory_handle: handle,
            _lock: lock,
        };
        let a = reservation(Uuid::new_v4(), &[3, 2, 1], 10);
        let overlapping = reservation(Uuid::new_v4(), &[4, 3, 2, 1], 11);
        let sibling = reservation(Uuid::new_v4(), &[5, 2, 1], 12);
        coordinator.claim(&a, Scope::Folder).unwrap();
        assert!(coordinator.owned(&a).unwrap());
        assert!(
            coordinator
                .claim(&overlapping, Scope::Folder)
                .unwrap_err()
                .contains("overlaps")
        );
        coordinator.claim(&sibling, Scope::Folder).unwrap();
        coordinator.release(&a).unwrap();
        coordinator.claim(&overlapping, Scope::Folder).unwrap();
        assert!(coordinator.owned(&overlapping).unwrap());
        assert!(coordinator.owned(&sibling).unwrap());
    }

    #[test]
    fn folder_and_process_reservations_are_independent() {
        let root = tempfile::tempdir().unwrap();
        let coordinator = Coordinator::for_test(root.path());
        let steam = reservation(Uuid::new_v4(), &[3, 2, 1], 10);
        let helper = reservation(Uuid::new_v4(), &[4, 3, 2, 1], 11);
        let same_process = reservation(Uuid::new_v4(), &[5, 2, 1], 11);
        coordinator.claim(&steam, Scope::Folder).unwrap();
        coordinator.claim(&steam, Scope::Process).unwrap();
        assert_eq!(
            coordinator.claim(&helper, Scope::Folder).unwrap_err(),
            OVERLAP
        );
        coordinator.claim(&helper, Scope::Process).unwrap();
        assert_eq!(
            coordinator
                .claim(&same_process, Scope::Process)
                .unwrap_err(),
            PROCESS_OVERLAP
        );
        assert!(coordinator.owns_scope(&steam, Scope::Folder).unwrap());
        assert!(coordinator.owns_scope(&steam, Scope::Process).unwrap());
        coordinator.release_scope(&steam, Scope::Folder).unwrap();
        assert!(!coordinator.owns_scope(&steam, Scope::Folder).unwrap());
        assert!(coordinator.owns_scope(&steam, Scope::Process).unwrap());
        assert!(coordinator.owns_scope(&helper, Scope::Process).unwrap());
        coordinator.claim(&helper, Scope::Folder).unwrap();
        coordinator.release_scope(&helper, Scope::Process).unwrap();
        assert!(coordinator.owns_scope(&helper, Scope::Folder).unwrap());
        assert!(!coordinator.owns_scope(&helper, Scope::Process).unwrap());
    }

    #[test]
    fn legacy_reservations_remain_owned_until_folder_restore() {
        let root = tempfile::tempdir().unwrap();
        let coordinator = Coordinator::for_test(root.path());
        let old = reservation(Uuid::new_v4(), &[3, 2, 1], 10);
        fs::write(
            coordinator.path(old.owner),
            serde_json::to_vec(&old).unwrap(),
        )
        .unwrap();
        assert!(coordinator.owns_scope(&old, Scope::Folder).unwrap());
        assert!(coordinator.owns_scope(&old, Scope::Process).unwrap());
        coordinator.release_scope(&old, Scope::Process).unwrap();
        assert!(coordinator.owns_scope(&old, Scope::Process).unwrap());
        coordinator.release_scope(&old, Scope::Folder).unwrap();
        assert!(!coordinator.owned(&old).unwrap());
    }
}
