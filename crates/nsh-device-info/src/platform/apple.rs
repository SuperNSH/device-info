//! The Apple bridge. Every iOS build — hardware, simulator (`cfg!(target_abi = "sim")`,
//! the cfg nsh-core also keys Catalyst's "macabi" off) and Catalyst — reads its model from
//! utsname's `machine` verbatim: hardware sees "iPhone16,2", a simulator sees the host's
//! arch ("arm64"), so a simulator never lies about being hardware, and its OS version from
//! UIDevice's `systemVersion`, the string NSProcessInfo's `operatingSystemVersion` numbers
//! produce. macOS reads `hw.model` ("Mac15,6") through sysctl and its version from os_info.
//! Both read app info from NSBundle's main bundle: no CFBundleIdentifier at all is
//! ERR_DEVICE_INFO_NOT_AVAILABLE, while a single missing key reads as the empty string —
//! package_info_plus's behavior. Every read is synchronous and nothing is cached.

use std::future::future;

use crate::{
    backend::{Backend, BackendFuture},
    error::{DeviceInfoError, ErrorKind},
    info::{AppInfo, Model, OsVersion},
};

#[cfg(target_os = "macos")]
use crate::info::from_os_info;

use objc2_foundation::{NSBundle, NSString};

// The marker's home is the objc2 crate; only this deprecated re-export is pinned.
#[cfg(target_os = "ios")]
use objc2_foundation::MainThreadMarker;

#[cfg(target_os = "ios")]
use objc2_ui_kit::UIDevice;

pub(crate) struct NativeBackend;

impl Backend for NativeBackend {
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
        Box::pin(future::ready(read_model()))
    }

    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
        Box::pin(future::ready(read_os_version()))
    }

    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
        Box::pin(future::ready(read_app_info()))
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn read_model() -> Result<Model, DeviceInfoError> {
    #[cfg(target_os = "ios")]
    {
        io_model()
    }
    #[cfg(target_os = "macos")]
    {
        mac_model()
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn read_os_version() -> Result<OsVersion, DeviceInfoError> {
    #[cfg(target_os = "ios")]
    {
        io_os_version()
    }
    #[cfg(target_os = "macos")]
    {
        Ok(from_os_info(&os_info::get()))
    }
}

/// visionOS and any other Apple target stay unhandled in v0.1: the query says so with the
/// NotAvailable code instead of guessing at a source.
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn read_model() -> Result<Model, DeviceInfoError> {
    Err(DeviceInfoError::new(ErrorKind::NotAvailable))
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn read_os_version() -> Result<OsVersion, DeviceInfoError> {
    Err(DeviceInfoError::new(ErrorKind::NotAvailable))
}

#[cfg(target_os = "ios")]
fn io_model() -> Result<Model, DeviceInfoError> {
    // SAFETY: an all-zero utsname is a valid buffer; nothing reads it before uname does.
    let mut un: libc::utsname = unsafe { std::mem::zeroed() };
    // libc declares uname safe; the valid-buffer contract above is this side's to keep.
    let named = libc::uname(&mut un) == 0;
    if !named {
        return Err(DeviceInfoError::new(ErrorKind::Backend).with_status(errno_status()));
    }
    let machine = c_string(&un.machine);
    // An empty machine string is missing data (None), never an error. UIKit offers no
    // machine of its own — its `model` only says "iPhone" — so name stays None in v0.1.
    Ok(Model { identifier: (!machine.is_empty()).then_some(machine), name: None })
}

#[cfg(target_os = "ios")]
fn io_os_version() -> Result<OsVersion, DeviceInfoError> {
    // UIDevice is main-thread only; any other thread gets the coded failure, not a hang.
    let Some(mtm) = MainThreadMarker::new() else {
        return Err(DeviceInfoError::new(ErrorKind::Backend));
    };
    let system_version = UIDevice::currentDevice(mtm).systemVersion().to_string();
    operating_system_version(&system_version)
        .ok_or_else(|| DeviceInfoError::new(ErrorKind::Backend))
}

/// The NSProcessInfo-version assembly: `systemVersion`'s "major[.minor[.patch]]" becomes
/// major and minor as numbers and `full` as "major.minor", the patch appended when
/// nonzero. Anything else is a backend failure.
#[cfg(any(target_os = "ios", test))]
fn operating_system_version(system_version: &str) -> Option<OsVersion> {
    let mut parts = system_version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u16>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let full = if patch == 0 {
        format!("{major}.{minor}")
    } else {
        format!("{major}.{minor}.{patch}")
    };
    Some(OsVersion { major, minor, full: Some(full) })
}

/// The one sysctl name this bridge asks for: the hardware model, not the arch.
#[cfg(any(target_os = "macos", test))]
const HW_MODEL_SYSCTL: &str = "hw.model";

/// The MIB sysctl wants for `name`; hw.model is the only name asked for here. hw.machine
/// is deliberately absent — utsname's machine already covers the arch-shaped answers.
#[cfg(any(target_os = "macos", test))]
fn sysctl_mib(name: &str) -> Option<[libc::c_int; 2]> {
    match name {
        HW_MODEL_SYSCTL => Some([libc::CTL_HW, libc::HW_MODEL]),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn mac_model() -> Result<Model, DeviceInfoError> {
    let Some(mut mib) = sysctl_mib(HW_MODEL_SYSCTL) else {
        return Err(DeviceInfoError::new(ErrorKind::Backend));
    };
    let mut model: [libc::c_char; 64] = [0; 64];
    let mut len = model.len();
    // libc declares sysctl safe; `mib` and `model` stay live and sized on this side, and
    // hw.model is a read-only string sysctl with no side effects.
    let status = libc::sysctl(
        mib.as_mut_ptr(),
        mib.len() as libc::c_uint,
        model.as_mut_ptr().cast(),
        &mut len,
        std::ptr::null_mut(),
        0,
    );
    if status != 0 {
        return Err(DeviceInfoError::new(ErrorKind::Backend).with_status(errno_status()));
    }
    let model = c_string(&model);
    Ok(Model { identifier: (!model.is_empty()).then_some(model), name: None })
}

/// NSBundle's release keys, spelled exactly as Apple's bundles spell them.
const IDENTIFIER_KEY: &str = "CFBundleIdentifier";
const VERSION_KEY: &str = "CFBundleShortVersionString";
const BUILD_KEY: &str = "CFBundleVersion";
const DISPLAY_NAME_KEY: &str = "CFBundleDisplayName";
const NAME_KEY: &str = "CFBundleName";

/// The bundle's value for `key` as a string; a missing key or a non-string value is None.
fn bundle_string(bundle: &NSBundle, key: &str) -> Option<String> {
    bundle
        .objectForInfoDictionaryKey(&NSString::from_str(key))
        .and_then(|value| value.downcast_ref::<NSString>().map(|s| s.to_string()))
}

fn read_app_info() -> Result<AppInfo, DeviceInfoError> {
    let bundle = NSBundle::mainBundle();
    let package_name = bundle_string(&bundle, IDENTIFIER_KEY);
    if package_name.is_none() {
        // No CFBundleIdentifier means no loadable main bundle (a bare executable): the
        // whole query is not available, not merely missing one key.
        return Err(DeviceInfoError::new(ErrorKind::NotAvailable));
    }
    // A missing individual key reads as the empty string, the way package_info_plus does.
    Ok(AppInfo {
        app_name: bundle_string(&bundle, DISPLAY_NAME_KEY)
            .or_else(|| bundle_string(&bundle, NAME_KEY))
            .unwrap_or_default(),
        package_name: package_name.unwrap_or_default(),
        version: bundle_string(&bundle, VERSION_KEY).unwrap_or_default(),
        build_number: bundle_string(&bundle, BUILD_KEY).unwrap_or_default(),
        build_signature: None,
        installer_store: None,
    })
}

/// The errno a failed uname/sysctl left behind, as the status a Backend error carries.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn errno_status() -> i64 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0) as i64
}

/// Reads the C string utsname or sysctl left in `bytes`, up to its first NUL byte.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn c_string(bytes: &[libc::c_char]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    // Identifiers are ASCII; lossy decoding keeps a stray byte from failing the read.
    let text: Vec<u8> = bytes[..end].iter().map(|&b| b as u8).collect();
    String::from_utf8_lossy(&text).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Machine strings a simulator's utsname holds: the host's CPU arch, never a hardware
    /// identifier. iOS reports utsname verbatim, so a simulator build answers with one of
    /// these — it never claims the hardware SIMULATOR_MODEL_IDENTIFIER would have named.
    fn is_simulator_machine(machine: &str) -> bool {
        matches!(machine, "i386" | "x86_64" | "arm64" | "arm64e")
    }

    #[test]
    fn a_simulator_reports_a_simulator_model() {
        assert!(is_simulator_machine("x86_64"));
        assert!(is_simulator_machine("arm64"));
        assert!(!is_simulator_machine("iPhone16,2"));
        assert!(!is_simulator_machine("Mac15,6"));
    }

    #[test]
    fn the_release_keys_name_the_right_fields() {
        assert_eq!(VERSION_KEY, "CFBundleShortVersionString");
        assert_eq!(BUILD_KEY, "CFBundleVersion");
        assert_eq!(DISPLAY_NAME_KEY, "CFBundleDisplayName");
        assert_eq!(NAME_KEY, "CFBundleName");
        assert_eq!(IDENTIFIER_KEY, "CFBundleIdentifier");
    }

    #[test]
    fn the_sysctl_name_resolves_to_the_hw_model_mib() {
        assert_eq!(sysctl_mib(HW_MODEL_SYSCTL), Some([libc::CTL_HW, libc::HW_MODEL]));
        assert_eq!(sysctl_mib("hw.machine"), None);
        assert_eq!(sysctl_mib("kern.ostype"), None);
    }

    #[test]
    fn operating_system_version_assembles_major_minor_patch() {
        let version = operating_system_version("18.1");
        assert_eq!(version.as_ref().map(|v| v.major), Some(18));
        assert_eq!(version.as_ref().map(|v| v.minor), Some(1));
        assert_eq!(version.as_ref().and_then(|v| v.full.as_deref()), Some("18.1"));

        let patched = operating_system_version("18.1.2");
        assert_eq!(patched.as_ref().map(|v| v.major), Some(18));
        assert_eq!(patched.as_ref().map(|v| v.minor), Some(1));
        assert_eq!(patched.as_ref().and_then(|v| v.full.as_deref()), Some("18.1.2"));

        assert!(operating_system_version("").is_none());
        assert!(operating_system_version("eighteen").is_none());
        assert!(operating_system_version("18.1.2.3").is_none());
    }
}
