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
    Foundation::HANDLE,
    Security::{
        EqualSid, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation,
        TOKEN_ELEVATION, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TOKEN_USER, TokenElevation,
        TokenIntegrityLevel, TokenSessionId, TokenUser,
    },
    System::{
        Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
        Threading::{
            CREATE_NEW_CONSOLE, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
            InitializeProcThreadAttributeList, OpenProcess, OpenProcessToken,
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

pub fn launch(path: &Path, directory: &Path) -> Result<(), String> {
    let current = token(unsafe { GetCurrentProcess() }, TOKEN_QUERY)?;
    let current_unprivileged = is_unprivileged(current.as_raw_handle())?;
    let (parent, shell) = checked_shell(!current_unprivileged)?;
    if current_unprivileged {
        return std::process::Command::new(path)
            .current_dir(directory)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Cannot start the game: {e}"));
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_context;

    #[test]
    fn refuses_wrong_user_session_or_elevated_desktop() {
        assert!(check_context(true, true, true).is_ok());
        assert!(check_context(false, true, true).is_err());
        assert!(check_context(true, false, true).is_err());
        assert!(check_context(true, true, false).is_err());
    }
}
