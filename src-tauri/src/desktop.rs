use crate::cpu::CpuPriority;
use std::{
    ffi::OsStr,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, SetLastError},
    Security::{
        AdjustTokenPrivileges, EqualSid, GetSidSubAuthority, GetSidSubAuthorityCount,
        GetTokenInformation, ImpersonateSelf, LookupPrivilegeValueW, RevertToSelf,
        SE_PRIVILEGE_ENABLED, SecurityImpersonation, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION,
        TOKEN_MANDATORY_LABEL, TOKEN_PRIVILEGES, TOKEN_QUERY, TOKEN_USER, TokenElevation,
        TokenIntegrityLevel, TokenSessionId, TokenUser,
    },
    System::{
        Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_PRIORITY_CLASS,
            JOBOBJECT_BASIC_LIMIT_INFORMATION, JobObjectBasicLimitInformation,
            SetInformationJobObject,
        },
        Threading::{
            CREATE_NEW_CONSOLE, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
            GetCurrentThread, GetPriorityClass, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
            InitializeProcThreadAttributeList, OpenProcess, OpenProcessToken, OpenThreadToken,
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, PROCESS_CREATE_PROCESS, PROCESS_INFORMATION,
            PROCESS_QUERY_LIMITED_INFORMATION, ResumeThread, STARTUPINFOEXW, TerminateProcess,
            UpdateProcThreadAttribute,
        },
    },
    UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId},
};

fn last_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}

fn token(process: HANDLE, access: u32) -> Result<OwnedHandle, String> {
    let mut handle = null_mut();
    if unsafe { OpenProcessToken(process, access, &mut handle) } == 0 {
        return Err(last_error("Cannot open process token"));
    }
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn token_info(handle: HANDLE, class: i32) -> Result<Vec<usize>, String> {
    let mut len = 0;
    unsafe { GetTokenInformation(handle, class, null_mut(), 0, &mut len) };
    if len == 0 {
        return Err(last_error("Cannot determine token information size"));
    }
    let mut buffer = vec![0usize; (len as usize).div_ceil(size_of::<usize>())];
    if unsafe { GetTokenInformation(handle, class, buffer.as_mut_ptr().cast(), len, &mut len) } == 0
    {
        return Err(last_error("Cannot read token information"));
    }
    Ok(buffer)
}

fn is_unprivileged(handle: HANDLE) -> Result<bool, String> {
    let elevation = token_info(handle, TokenElevation)?;
    let label = token_info(handle, TokenIntegrityLevel)?;
    let sid = unsafe {
        (*(label.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()))
            .Label
            .Sid
    };
    let count = unsafe { *GetSidSubAuthorityCount(sid) };
    if count == 0 {
        return Err("The token has no integrity level".into());
    }
    let integrity = unsafe { *GetSidSubAuthority(sid, u32::from(count - 1)) };
    Ok(
        unsafe { (*(elevation.as_ptr().cast::<TOKEN_ELEVATION>())).TokenIsElevated } == 0
            && integrity < 0x3000,
    )
}

fn check_context(same_user: bool, same_session: bool, unprivileged: bool) -> Result<(), String> {
    if !same_user {
        return Err("The administrator account differs from the signed-in desktop user".into());
    }
    if !same_session {
        return Err("The desktop belongs to a different Windows session".into());
    }
    if !unprivileged {
        return Err("The Windows desktop is elevated".into());
    }
    Ok(())
}

fn check_token_context(candidate: HANDLE, current: HANDLE) -> Result<(), String> {
    let shell_user = token_info(candidate, TokenUser)?;
    let current_user = token_info(current, TokenUser)?;
    let shell_sid = unsafe { (*(shell_user.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let current_sid = unsafe { (*(current_user.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let same_user = unsafe { EqualSid(shell_sid, current_sid) } != 0;
    let shell_session = token_info(candidate, TokenSessionId)?;
    let current_session = token_info(current, TokenSessionId)?;
    let same_session =
        unsafe { *shell_session.as_ptr().cast::<u32>() == *current_session.as_ptr().cast::<u32>() };
    check_context(same_user, same_session, is_unprivileged(candidate)?)
}

fn checked_shell(create_process: bool) -> Result<(OwnedHandle, OwnedHandle), String> {
    let window = unsafe { GetShellWindow() };
    if window.is_null() {
        return Err("Cannot find the Windows desktop".into());
    }
    let mut pid = 0;
    if unsafe { GetWindowThreadProcessId(window, &mut pid) } == 0 || pid == 0 {
        return Err(last_error("Cannot identify the Windows desktop process"));
    }
    let access = PROCESS_QUERY_LIMITED_INFORMATION
        | if create_process {
            PROCESS_CREATE_PROCESS
        } else {
            0
        };
    let process = unsafe { OpenProcess(access, 0, pid) };
    if process.is_null() {
        return Err(last_error("Cannot open the Windows desktop process"));
    }
    let process = unsafe { OwnedHandle::from_raw_handle(process) };
    let shell = token(process.as_raw_handle(), TOKEN_QUERY)?;
    let current = token(unsafe { GetCurrentProcess() }, TOKEN_QUERY)?;
    check_token_context(shell.as_raw_handle(), current.as_raw_handle())?;
    Ok((process, shell))
}

pub fn check_user() -> Result<(), String> {
    checked_shell(false).map(drop)
}

struct Environment(*mut std::ffi::c_void);

impl Drop for Environment {
    fn drop(&mut self) {
        unsafe { DestroyEnvironmentBlock(self.0) };
    }
}

fn priority_job(priority: CpuPriority) -> Result<OwnedHandle, String> {
    std::thread::spawn(move || {
        if unsafe { ImpersonateSelf(SecurityImpersonation) } == 0 {
            return Err(last_error(
                "Cannot impersonate current user for priority job",
            ));
        }
        let result = enable_priority_privilege().and_then(|()| create_priority_job(priority));
        if unsafe { RevertToSelf() } == 0 {
            return Err(last_error("Cannot release priority privilege"));
        }
        result
    })
    .join()
    .map_err(|_| "Priority job setup thread panicked".to_string())?
}

fn enable_priority_privilege() -> Result<(), String> {
    let mut raw = null_mut();
    if unsafe {
        OpenThreadToken(
            GetCurrentThread(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            1,
            &mut raw,
        )
    } == 0
    {
        return Err(last_error("Cannot open priority privilege token"));
    }
    let token = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut privileges = TOKEN_PRIVILEGES::default();
    privileges.PrivilegeCount = 1;
    let name: Vec<u16> = "SeIncreaseBasePriorityPrivilege\0".encode_utf16().collect();
    if unsafe { LookupPrivilegeValueW(null(), name.as_ptr(), &mut privileges.Privileges[0].Luid) }
        == 0
    {
        return Err(last_error("Cannot look up priority privilege"));
    }
    privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
    unsafe { SetLastError(0) };
    if unsafe {
        AdjustTokenPrivileges(
            token.as_raw_handle(),
            0,
            &privileges,
            0,
            null_mut(),
            null_mut(),
        )
    } == 0
    {
        return Err(last_error("Cannot enable priority privilege"));
    }
    if unsafe { GetLastError() } == ERROR_NOT_ALL_ASSIGNED {
        return Err("Priority privilege is not available in the current token".into());
    }
    Ok(())
}

fn create_priority_job(priority: CpuPriority) -> Result<OwnedHandle, String> {
    let handle = unsafe { CreateJobObjectW(null(), null()) };
    if handle.is_null() {
        return Err(last_error("Cannot create priority job"));
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut limits: JOBOBJECT_BASIC_LIMIT_INFORMATION = unsafe { zeroed() };
    limits.LimitFlags = JOB_OBJECT_LIMIT_PRIORITY_CLASS;
    limits.PriorityClass = priority_class(priority);
    if unsafe {
        SetInformationJobObject(
            handle.as_raw_handle(),
            JobObjectBasicLimitInformation,
            (&limits as *const JOBOBJECT_BASIC_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_BASIC_LIMIT_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(last_error("Cannot configure priority job"));
    }
    Ok(handle)
}

fn priority_class(priority: CpuPriority) -> u32 {
    match priority {
        CpuPriority::Low => IDLE_PRIORITY_CLASS,
        CpuPriority::High => HIGH_PRIORITY_CLASS,
    }
}

fn resume(
    process: &OwnedHandle,
    thread: &OwnedHandle,
    job: Option<(&OwnedHandle, CpuPriority)>,
) -> Result<bool, String> {
    let job_failed = job.is_some_and(|(job, priority)| {
        if unsafe { AssignProcessToJobObject(job.as_raw_handle(), process.as_raw_handle()) } == 0 {
            log::warn!("{}", last_error("Cannot assign process to priority job"));
            true
        } else if unsafe { GetPriorityClass(process.as_raw_handle()) } != priority_class(priority) {
            log::warn!("The process priority does not match the requested job priority");
            true
        } else {
            false
        }
    });
    if unsafe { ResumeThread(thread.as_raw_handle()) } != 1 {
        let detail = last_error("Cannot resume the verified game");
        if unsafe { TerminateProcess(process.as_raw_handle(), 1) } == 0 {
            return Err(format!(
                "{detail}; {}",
                last_error("Cannot terminate suspended game")
            ));
        }
        return Err(detail);
    }
    Ok(job_failed)
}

pub fn launch(
    path: &Path,
    directory: &Path,
    priority: Option<CpuPriority>,
) -> Result<bool, String> {
    let job = match priority.map(priority_job).transpose() {
        Ok(job) => job,
        Err(detail) => {
            log::warn!("{detail}");
            launch(path, directory, None)?;
            return Ok(true);
        }
    };
    let current = token(unsafe { GetCurrentProcess() }, TOKEN_QUERY)?;
    let current_unprivileged = is_unprivileged(current.as_raw_handle())?;
    let (parent, shell) = checked_shell(!current_unprivileged)?;
    if current_unprivileged && job.is_none() {
        return std::process::Command::new(path)
            .current_dir(directory)
            .spawn()
            .map(|_| false)
            .map_err(|e| format!("Cannot start the game: {e}"));
    }
    if current_unprivileged {
        let wide = |value: &OsStr| {
            value
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>()
        };
        let file = wide(path.as_os_str());
        let mut command = vec![b'"' as u16];
        command.extend(path.as_os_str().encode_wide());
        command.extend([b'"' as u16, 0]);
        let directory = wide(directory.as_os_str());
        let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
        startup.StartupInfo.cb =
            size_of::<windows_sys::Win32::System::Threading::STARTUPINFOW>() as u32;
        let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
        if unsafe {
            CreateProcessW(
                file.as_ptr(),
                command.as_mut_ptr(),
                null(),
                null(),
                0,
                CREATE_SUSPENDED,
                null(),
                directory.as_ptr(),
                &startup.StartupInfo,
                &mut process,
            )
        } == 0
        {
            return Err(last_error("Cannot start the game"));
        }
        let thread = unsafe { OwnedHandle::from_raw_handle(process.hThread) };
        let process = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
        return resume(&process, &thread, job.as_ref().zip(priority));
    }
    let mut environment = null_mut();
    if unsafe { CreateEnvironmentBlock(&mut environment, shell.as_raw_handle(), 0) } == 0 {
        return Err(last_error("Cannot prepare the desktop user environment"));
    }
    let environment = Environment(environment);
    // The desktop process supplies the child token; verify it before allowing any code to run.
    let mut size = 0;
    unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size) };
    if size == 0 {
        return Err(last_error("Cannot size process attributes"));
    }
    let mut storage = vec![0usize; size.div_ceil(size_of::<usize>())];
    let attributes = storage.as_mut_ptr().cast();
    if unsafe { InitializeProcThreadAttributeList(attributes, 1, 0, &mut size) } == 0 {
        return Err(last_error("Cannot initialize process attributes"));
    }
    struct Attributes(*mut std::ffi::c_void);
    impl Drop for Attributes {
        fn drop(&mut self) {
            unsafe { DeleteProcThreadAttributeList(self.0) };
        }
    }
    let attributes = Attributes(attributes);
    let parent_handle = parent.as_raw_handle();
    if unsafe {
        UpdateProcThreadAttribute(
            attributes.0,
            0,
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
            (&parent_handle as *const HANDLE).cast(),
            size_of::<HANDLE>(),
            null_mut(),
            null(),
        )
    } == 0
    {
        return Err(last_error("Cannot set desktop process as parent"));
    }
    let wide = |value: &OsStr| {
        value
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let file = wide(path.as_os_str());
    let mut command = vec![b'"' as u16];
    command.extend(path.as_os_str().encode_wide());
    command.extend([b'"' as u16, 0]);
    let directory = wide(directory.as_os_str());
    let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attributes.0;
    let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
    if unsafe {
        CreateProcessW(
            file.as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            0,
            CREATE_UNICODE_ENVIRONMENT
                | EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_NEW_CONSOLE,
            environment.0,
            directory.as_ptr(),
            &startup.StartupInfo,
            &mut process,
        )
    } == 0
    {
        return Err(last_error(
            "Cannot start the game without administrator privileges",
        ));
    }
    let thread = unsafe { OwnedHandle::from_raw_handle(process.hThread) };
    let process = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
    let verification = token(process.as_raw_handle(), TOKEN_QUERY)
        .and_then(|child| check_token_context(child.as_raw_handle(), current.as_raw_handle()));
    if let Err(detail) = verification {
        if unsafe { TerminateProcess(process.as_raw_handle(), 1) } == 0 {
            return Err(format!(
                "{detail}; {}",
                last_error("Cannot terminate suspended game")
            ));
        }
        return Err(format!("Game token verification failed: {detail}"));
    }
    resume(&process, &thread, job.as_ref().zip(priority))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_wrong_user_session_or_elevated_desktop() {
        assert!(check_context(true, true, true).is_ok());
        assert!(check_context(false, true, true).is_err());
        assert!(check_context(true, false, true).is_err());
        assert!(check_context(true, true, false).is_err());
    }

    #[test]
    #[ignore = "compiles and launches a disposable executable without the priority privilege"]
    fn launch_warns_and_continues_without_priority_privilege() {
        assert!(priority_job(CpuPriority::Low).is_err());
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("game.exe");
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("launch_dummy.rs");
        assert!(
            std::process::Command::new("rustc")
                .arg(source)
                .arg("-o")
                .arg(&executable)
                .status()
                .unwrap()
                .success()
        );
        assert!(launch(&executable, dir.path(), Some(CpuPriority::Low)).unwrap());
        let marker = dir.path().join("launch-marker.txt");
        for _ in 0..200 {
            if marker.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(marker.exists());
    }

    #[test]
    #[ignore = "compiles and launches disposable process trees"]
    fn priority_job_applies_to_parent_and_child() {
        use windows_sys::Win32::System::Threading::{
            GetPriorityClass, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            WaitForSingleObject,
        };

        struct Stop {
            file: std::path::PathBuf,
            process: Option<OwnedHandle>,
        }
        impl Drop for Stop {
            fn drop(&mut self) {
                if let Err(e) = std::fs::write(&self.file, b"stop") {
                    eprintln!("Cannot stop disposable process tree: {e}");
                }
                if let Some(process) = &self.process {
                    if unsafe { WaitForSingleObject(process.as_raw_handle(), 5000) } != 0 {
                        eprintln!("Disposable process tree did not exit in time");
                    }
                }
            }
        }

        for (priority, expected) in [
            (CpuPriority::Low, IDLE_PRIORITY_CLASS),
            (CpuPriority::High, HIGH_PRIORITY_CLASS),
        ] {
            drop(priority_job(priority).expect("priority job must be available"));
            let dir = tempfile::tempdir().unwrap();
            let mut stop = Stop {
                file: dir.path().join("stop"),
                process: None,
            };
            let executable = dir.path().join("tree.exe");
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("tests")
                .join("fixtures")
                .join("priority_tree.rs");
            assert!(
                std::process::Command::new("rustc")
                    .arg(source)
                    .arg("-o")
                    .arg(&executable)
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(!launch(&executable, dir.path(), Some(priority)).unwrap());
            let marker = dir.path().join("pids");
            for _ in 0..200 {
                if marker.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            let pids = std::fs::read_to_string(marker).unwrap();
            let pids: Vec<u32> = pids
                .split_whitespace()
                .map(|pid| pid.parse().unwrap())
                .collect();
            assert_eq!(pids.len(), 2);
            for (index, pid) in pids.into_iter().enumerate() {
                let process = unsafe {
                    OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                        0,
                        pid,
                    )
                };
                assert!(!process.is_null());
                let process = unsafe { OwnedHandle::from_raw_handle(process) };
                assert_eq!(
                    unsafe { GetPriorityClass(process.as_raw_handle()) },
                    expected
                );
                if index == 0 {
                    stop.process = Some(process);
                }
            }
        }
    }
}
