use crate::{AppError, error, executable};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::SetLastError,
    Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW},
};

fn query<'a>(data: &'a [u8], key: &str, unit_size: usize) -> Result<Option<&'a [u8]>, AppError> {
    let wide: Vec<u16> = OsStr::new(key).encode_wide().chain(Some(0)).collect();
    let mut pointer = std::ptr::null_mut();
    let mut length = 0u32;
    if unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            wide.as_ptr(),
            &mut pointer,
            &mut length,
        )
    } == 0
    {
        return Ok(None);
    }
    let start = (pointer as usize)
        .checked_sub(data.as_ptr() as usize)
        .ok_or_else(|| error("NAME_READ", "Invalid version resource pointer"))?;
    let size = (length as usize)
        .checked_mul(unit_size)
        .ok_or_else(|| error("NAME_READ", "Invalid version resource length"))?;
    let end = start
        .checked_add(size)
        .ok_or_else(|| error("NAME_READ", "Invalid version resource length"))?;
    data.get(start..end)
        .map(Some)
        .ok_or_else(|| error("NAME_READ", "Version resource is outside its buffer"))
}

fn version_name(path: &Path) -> Result<Option<String>, AppError> {
    log::info!("Reading executable version metadata");
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe { SetLastError(0) };
    let size = unsafe { GetFileVersionInfoSizeW(wide.as_ptr(), std::ptr::null_mut()) };
    if size == 0 {
        let cause = std::io::Error::last_os_error();
        return match cause.raw_os_error() {
            Some(0 | 1812 | 1813 | 1814 | 1815) => {
                log::info!("Executable has no version resource");
                Ok(None)
            }
            _ => Err(error("NAME_READ", cause)),
        };
    }
    if size > 16 * 1024 * 1024 {
        return Err(error(
            "NAME_READ",
            "Executable version resource is too large",
        ));
    }
    let mut data = vec![0u8; size as usize];
    if unsafe { GetFileVersionInfoW(wide.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return Err(error("NAME_READ", std::io::Error::last_os_error()));
    }
    let mut translations = Vec::new();
    if let Some(bytes) = query(&data, r"\VarFileInfo\Translation", 1)? {
        if bytes.len() % 4 != 0 {
            return Err(error("NAME_READ", "Invalid version translations"));
        }
        for pair in bytes.chunks_exact(4) {
            translations.push((
                u16::from_le_bytes([pair[0], pair[1]]),
                u16::from_le_bytes([pair[2], pair[3]]),
            ));
        }
    }
    for fallback in [(0x0409, 0x04b0), (0x0409, 0x04e4)] {
        if !translations.contains(&fallback) {
            translations.push(fallback);
        }
    }
    for field in ["ProductName", "FileDescription"] {
        for &(language, codepage) in &translations {
            let key = format!(r"\StringFileInfo\{language:04x}{codepage:04x}\{field}");
            if let Some(bytes) = query(&data, &key, 2)? {
                let words: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|p| u16::from_le_bytes([p[0], p[1]]))
                    .collect();
                let end = words
                    .iter()
                    .position(|&word| word == 0)
                    .unwrap_or(words.len());
                let value = String::from_utf16(&words[..end]).map_err(|e| error("NAME_READ", e))?;
                let value = value.trim();
                if !value.is_empty() {
                    return Ok(Some(value.into()));
                }
            }
        }
    }
    log::info!("Executable version resource has no product name or description");
    Ok(None)
}

pub fn read(path: &str) -> Result<Option<String>, AppError> {
    version_name(&executable(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_product_metadata_from_windows_binary() {
        let system = std::env::var_os("SystemRoot").expect("SystemRoot must be set");
        let path = Path::new(&system).join("System32").join("kernel32.dll");
        let name = version_name(&path)
            .unwrap()
            .expect("Windows binary has version metadata");
        assert!(!name.trim().is_empty());
    }

    #[test]
    #[ignore = "requires an explicitly provided executable without version metadata"]
    fn executable_without_metadata_has_no_name() {
        let exe = std::env::var("GAME_OPTIMIZER_DUMMY_EXE").expect("Set GAME_OPTIMIZER_DUMMY_EXE");
        assert_eq!(read(&exe).unwrap(), None);
    }
}
