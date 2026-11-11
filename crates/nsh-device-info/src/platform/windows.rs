//! Windows: the firmware's `SystemProductName` registry value is the device model, `os_info`
//! the OS version, and the running exe's version resource the app version.
//!
//! The Win32 reads (registry, version resource) are platform-only; the parsers under them are
//! pure and host-portable. Windows has no bundle id, so `package_name` is the exe file name
//! without its extension, which the parity doc marks Partial.

use std::{env, ffi::c_void, iter::once, os::windows::ffi::OsStrExt, path::Path, ptr};

use windows::{
    Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR},
    Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW},
    Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW},
    core::PCWSTR,
};

use crate::{
    backend::{Backend, BackendFuture},
    error::{DeviceInfoError, ErrorKind},
    info::{AppInfo, Model, OsVersion},
};

pub(crate) struct NativeBackend;

/// The key the firmware publishes the DMI product name under.
const BIOS_KEY: &str = r"HARDWARE\DESCRIPTION\System\BIOS";

/// The value holding the model string, exactly as the firmware wrote it.
const SYSTEM_PRODUCT_NAME: &str = "SystemProductName";

/// The version resource's language/codepage table; its first entry locates every string.
const TRANSLATION_BLOCK: &str = r"\VarFileInfo\Translation";

const FILE_VERSION: &str = "FileVersion";
const PRODUCT_NAME: &str = "ProductName";

/// A NUL-terminated UTF-16 buffer, as the wide Win32 APIs take their paths and names.
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(once(0)).collect()
}

/// The wide buffer up to its NUL terminator; the registry and the version resource both
/// return NUL-terminated strings.
fn string_from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// `\StringFileInfo\<lang><codepage>\<name>` with both halves in four hex digits.
fn string_block(language: u16, codepage: u16, name: &str) -> String {
    format!(r"\StringFileInfo\{language:04x}{codepage:04x}\{name}")
}

/// The build number is the `FileVersion`'s last dot-separated part.
fn last_version_part(version: &str) -> &str {
    version.split('.').next_back().unwrap_or("")
}

/// The final path component without its extension; Windows has no bundle id, so the exe
/// name stands in for `package_name`.
fn file_stem(exe: &Path) -> String {
    exe.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

/// `ProductName` when the resource carries one, otherwise the exe's file stem.
fn app_name(product_name: &str, exe: &Path) -> String {
    if product_name.is_empty() {
        file_stem(exe)
    } else {
        product_name.to_owned()
    }
}

/// The first language/codepage pair of a translation table; the resource stores little-endian
/// words, and every Windows target is little-endian.
fn first_translation(table: &[u8]) -> Option<(u16, u16)> {
    if table.len() < 4 {
        return None;
    }
    let language = u16::from_le_bytes([table[0], table[1]]);
    let codepage = u16::from_le_bytes([table[2], table[3]]);
    Some((language, codepage))
}

/// Every registry failure that is not a missing value carries the OS error code as status.
fn backend_error(code: WIN32_ERROR) -> DeviceInfoError {
    DeviceInfoError::new(ErrorKind::Backend).with_status(i64::from(code.0))
}

/// A version resource this bridge cannot read at all.
fn not_available() -> DeviceInfoError {
    DeviceInfoError::new(ErrorKind::NotAvailable)
}

/// Reads the BIOS `SystemProductName`. A missing value is partial data, not an error;
/// Windows reports no marketing name to fill `Model::name`.
fn read_model() -> Result<Model, DeviceInfoError> {
    let subkey = wide(BIOS_KEY);
    let value = wide(SYSTEM_PRODUCT_NAME);
    let mut size = 0u32;
    // SAFETY: both wide buffers outlive the call; no data buffer, so the API only sizes the read.
    let probed = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR::from_raw(subkey.as_ptr()),
            PCWSTR::from_raw(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    if probed == ERROR_FILE_NOT_FOUND {
        return Ok(Model {
            identifier: None,
            name: None,
        });
    }
    if probed != ERROR_SUCCESS {
        return Err(backend_error(probed));
    }
    let mut buffer = vec![0u16; size.div_ceil(2) as usize];
    let mut bytes = (buffer.len() * 2) as u32;
    // SAFETY: `buffer` owns `bytes` zeroed bytes, more than the sized read can write.
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR::from_raw(subkey.as_ptr()),
            PCWSTR::from_raw(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            Some(&mut bytes),
        )
    };
    if read == ERROR_FILE_NOT_FOUND {
        return Ok(Model {
            identifier: None,
            name: None,
        });
    }
    if read != ERROR_SUCCESS {
        return Err(backend_error(read));
    }
    let product = string_from_wide(&buffer);
    Ok(Model {
        identifier: (!product.is_empty()).then_some(product),
        name: None,
    })
}

/// `os_info` never fails: a release it cannot parse answers `Unknown`, which is `(0, 0, None)`.
fn current_os_version() -> OsVersion {
    OsVersion::from_os_info(&os_info::get())
}

/// One string from the version resource; a missing string is empty, as package_info_plus answers.
fn version_string(block: &[u8], language: u16, codepage: u16, name: &str) -> String {
    let subblock = wide(&string_block(language, codepage, name));
    let mut value: *mut c_void = ptr::null_mut();
    let mut chars = 0u32;
    // SAFETY: `subblock` is NUL-terminated and outlives the call.
    let queried = unsafe {
        VerQueryValueW(
            block.as_ptr().cast::<c_void>(),
            PCWSTR::from_raw(subblock.as_ptr()),
            &mut value,
            &mut chars,
        )
    };
    if !queried.as_bool() || value.is_null() || chars == 0 {
        return String::new();
    }
    // SAFETY: a StringFileInfo value points at `chars` WCHARs inside `block`, NUL included.
    let wide_value = unsafe { std::slice::from_raw_parts(value.cast::<u16>(), chars as usize) };
    string_from_wide(wide_value)
}

/// The exe's `FileVersion` and `ProductName`; an unreadable resource is `NotAvailable` outright.
fn version_strings(exe: &Path) -> Result<(String, String), DeviceInfoError> {
    let path: Vec<u16> = exe.as_os_str().encode_wide().chain(once(0)).collect();
    let filename = PCWSTR::from_raw(path.as_ptr());
    // SAFETY: `path` is NUL-terminated and outlives the call.
    let size = unsafe { GetFileVersionInfoSizeW(filename, None) };
    if size == 0 {
        return Err(not_available());
    }
    let mut block = vec![0u8; size as usize];
    // SAFETY: `block` owns exactly the `size` bytes the API reads the resource into.
    unsafe {
        GetFileVersionInfoW(filename, None, size, block.as_mut_ptr().cast::<c_void>())
            .map_err(|_| not_available())?;
    }
    let mut table: *mut c_void = ptr::null_mut();
    let mut table_bytes = 0u32;
    let subblock = wide(TRANSLATION_BLOCK);
    // SAFETY: `subblock` is NUL-terminated; the returned pointer stays inside `block`.
    let queried = unsafe {
        VerQueryValueW(
            block.as_ptr().cast::<c_void>(),
            PCWSTR::from_raw(subblock.as_ptr()),
            &mut table,
            &mut table_bytes,
        )
    };
    let translation = if queried.as_bool() && !table.is_null() {
        // SAFETY: VerQueryValueW points `table` at `table_bytes` bytes inside `block`.
        Some(unsafe { std::slice::from_raw_parts(table.cast::<u8>(), table_bytes as usize) })
    } else {
        None
    };
    let (language, codepage) = translation
        .and_then(first_translation)
        .ok_or_else(not_available)?;
    Ok((
        version_string(&block, language, codepage, FILE_VERSION),
        version_string(&block, language, codepage, PRODUCT_NAME),
    ))
}

/// The running exe's version resource answers the app's own version fields.
fn read_app_info() -> Result<AppInfo, DeviceInfoError> {
    // windows 0.62 gates GetModuleFileNameW behind Win32_System_LibraryLoader, which the pinned
    // features omit; current_exe is that same read of the running exe's path.
    let exe = env::current_exe().map_err(|_| not_available())?;
    let (version, product_name) = version_strings(&exe)?;
    let build_number = last_version_part(&version).to_owned();
    Ok(AppInfo {
        app_name: app_name(&product_name, &exe),
        package_name: file_stem(&exe),
        version,
        build_number,
        build_signature: None,
        installer_store: None,
    })
}

impl Backend for NativeBackend {
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
        Box::pin(async { read_model() })
    }

    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
        Box::pin(async { Ok(current_os_version()) })
    }

    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
        Box::pin(async { read_app_info() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_release_keys_name_the_right_fields() {
        assert_eq!(BIOS_KEY, r"HARDWARE\DESCRIPTION\System\BIOS");
        assert_eq!(SYSTEM_PRODUCT_NAME, "SystemProductName");
        assert_eq!(TRANSLATION_BLOCK, r"\VarFileInfo\Translation");
        assert_eq!(FILE_VERSION, "FileVersion");
        assert_eq!(PRODUCT_NAME, "ProductName");
    }

    #[test]
    fn a_string_block_spells_the_first_translation_table_entry() {
        assert_eq!(
            string_block(0x0409, 0x04b0, FILE_VERSION),
            r"\StringFileInfo\040904b0\FileVersion"
        );
        assert_eq!(
            string_block(0x0804, 0x04e4, PRODUCT_NAME),
            r"\StringFileInfo\080404e4\ProductName"
        );
    }

    #[test]
    fn wide_strings_stop_at_the_nul_terminator() {
        let terminated: Vec<u16> = "OptiPlex 7090"
            .encode_utf16()
            .chain(once(0))
            .chain("tail".encode_utf16())
            .collect();
        assert_eq!(string_from_wide(&terminated), "OptiPlex 7090");
        assert_eq!(string_from_wide(&[0]), "");
        assert_eq!(string_from_wide(&[]), "");
    }

    #[test]
    fn the_build_number_is_the_last_part_of_the_file_version() {
        assert_eq!(last_version_part("1.2.3.4"), "4");
        assert_eq!(last_version_part("6.3"), "3");
        assert_eq!(last_version_part("0.0.1"), "1");
        assert_eq!(last_version_part("0.1.0-dev"), "0-dev");
        assert_eq!(last_version_part(""), "");
        assert_eq!(last_version_part("1,2,3"), "1,2,3");
    }

    #[test]
    fn the_translation_table_reduces_to_its_first_entry() {
        // 0x0409 (en-US), codepage 1252: 0x04e4 little-endian.
        assert_eq!(
            first_translation(&[0x09, 0x04, 0xe4, 0x04]),
            Some((0x0409, 1252))
        );
        assert_eq!(first_translation(&[]), None);
        assert_eq!(first_translation(&[0x09, 0x04]), None);
        assert_eq!(first_translation(&[0x09, 0x04, 0xe4]), None);
    }

    #[test]
    fn app_name_falls_back_to_the_exe_stem_when_product_name_is_missing() {
        let exe = Path::new(r"C:\Program Files\NSH\nsh app.exe");
        assert_eq!(app_name("NSH App", exe), "NSH App");
        assert_eq!(app_name("", exe), "nsh app");
        assert_eq!(app_name("", Path::new("app.exe")), "app");
    }

    #[test]
    fn package_name_is_the_exe_file_name_without_extension() {
        assert_eq!(file_stem(Path::new(r"C:\Program Files\NSH\app.exe")), "app");
        assert_eq!(file_stem(Path::new("nsh.exe")), "nsh");
    }

    #[test]
    fn an_unknown_os_info_answer_is_all_zeros() {
        let version = OsVersion::from_os_info(&os_info::Info::unknown());
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 0);
        assert_eq!(version.full, None);
    }
}
