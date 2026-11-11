//! Android: `android.os.Build` and `PackageManager` are read over JNI, with the VM and the
//! application adopted from `ndk-context` or installed into it once, the way the biometrics
//! crate does. The JNI reads run only on an Android target; every decision they rest on —
//! class, field and method names, the version assembly, the digest formatting — lives in the
//! plain helpers below with inline tests that need no device.

use std::{
    ffi::{c_char, c_int, c_void},
    future::ready,
    mem::transmute,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::null_mut,
    sync::Mutex,
};

use jni::{
    Env, JavaVM, jni_sig,
    objects::{JByteArray, JObject, JObjectArray, JString, JValue},
    strings::JNIString,
    sys::{self, jint, jsize},
};

use crate::{
    backend::{Backend, BackendFuture},
    error::{DeviceInfoError, ErrorKind},
    info::{AppInfo, Model, OsVersion},
};

pub(crate) struct NativeBackend;

/// `android.os.Build.MODEL`: the raw hardware model string, e.g. "SM-S928B".
const BUILD_CLASS: &str = "android/os/Build";
const MODEL_FIELD: &str = "MODEL";
/// `android.os.Build.VERSION`: the API level and the marketing release, e.g. 34 and "14".
const VERSION_CLASS: &str = "android/os/Build$VERSION";
const SDK_INT_FIELD: &str = "SDK_INT";
const RELEASE_FIELD: &str = "RELEASE";
/// `ActivityThread.currentApplication` yields the application, the Context reads go through.
const ACTIVITY_THREAD_CLASS: &str = "android/app/ActivityThread";
const CURRENT_APPLICATION: &str = "currentApplication";
const GET_PACKAGE_MANAGER: &str = "getPackageManager";
const GET_PACKAGE_NAME: &str = "getPackageName";
const GET_PACKAGE_INFO: &str = "getPackageInfo";
const VERSION_NAME_FIELD: &str = "versionName";
const VERSION_CODE_FIELD: &str = "versionCode";
const LONG_VERSION_CODE: &str = "getLongVersionCode";
const APPLICATION_INFO_FIELD: &str = "applicationInfo";
const LOAD_LABEL: &str = "loadLabel";
const TO_STRING: &str = "toString";
/// InstallSourceInfo (API 30+) or PackageManager (older) name the store that installed the app.
const GET_INSTALL_SOURCE_INFO: &str = "getInstallSourceInfo";
const INITIATING_PACKAGE_NAME: &str = "getInitiatingPackageName";
const GET_INSTALLER_PACKAGE_NAME: &str = "getInstallerPackageName";
/// Signing: SigningInfo from API 28, the legacy signatures array before it.
const SIGNING_INFO_FIELD: &str = "signingInfo";
const HAS_MULTIPLE_SIGNERS: &str = "hasMultipleSigners";
const APK_CONTENTS_SIGNERS: &str = "apkContentsSigners";
const SIGNING_CERTIFICATE_HISTORY: &str = "signingCertificateHistory";
const SIGNATURES_FIELD: &str = "signatures";
const TO_BYTE_ARRAY: &str = "toByteArray";
const MESSAGE_DIGEST_CLASS: &str = "java/security/MessageDigest";
const GET_INSTANCE: &str = "getInstance";
const DIGEST: &str = "digest";
const SHA256: &str = "SHA-256";
/// `PackageManager.GET_SIGNATURES` and `GET_SIGNING_CERTIFICATES`, the two signer flags.
const GET_SIGNATURES: jint = 0x0000_0040;
const GET_SIGNING_CERTIFICATES: jint = 0x0800_0000;
/// `Build.VERSION_CODES.P`: `getLongVersionCode` and SigningInfo start at API 28.
const SDK_P: jint = 28;
/// `Build.VERSION_CODES.R`: `getInstallSourceInfo` starts at API 30.
const SDK_R: jint = 30;

/// `android.content.pm.Signature[]`, the array both signing APIs return; only element zero is read.
type SignatureArray<'local> = JObjectArray<'local, JObject<'local>>;

/// `JNI_GetCreatedJavaVMs`, found by name in `libnativehelper`.
type GetCreatedVms = unsafe extern "C" fn(*mut *mut sys::JavaVM, jsize, *mut jsize) -> jint;

// libdl's two loaders, linked by default on Android; declaring the symbols here keeps the
// android target to its pinned jni and ndk-context dependencies.
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// `RTLD_NOW` from bionic's `dlfcn.h`.
const RTLD_NOW: c_int = 2;

/// Set once `ndk-context` holds the VM and the application. It can be set only once.
static CONTEXT: Mutex<bool> = Mutex::new(false);

impl From<jni::errors::Error> for DeviceInfoError {
    fn from(_: jni::errors::Error) -> Self {
        DeviceInfoError::new(ErrorKind::Backend)
    }
}

/// nsh-core's Android convention: the SDK level is the major, the minor is zero, and RELEASE
/// is the full version.
fn assemble_os_version(sdk: jint, release: Option<String>) -> OsVersion {
    OsVersion {
        major: sdk.clamp(0, u16::MAX as jint) as u16,
        minor: 0,
        full: release,
    }
}

/// Upper-case hex, the format package_info_plus gives a signing certificate's digest.
fn hex_uppercase(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        hex.push(DIGITS[(byte >> 4) as usize] as char);
        hex.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    hex
}

/// The VM of this process. An NSH app has no JVM of its own, so it is found through the runtime's
/// library. A missing VM is not available, not a backend failure.
fn find_vm() -> Result<*mut sys::JavaVM, DeviceInfoError> {
    // SAFETY: the names are valid C strings, and the symbol is called with the signature
    // the JNI specification gives it.
    unsafe {
        let library = dlopen(c"libnativehelper.so".as_ptr(), RTLD_NOW);
        if library.is_null() {
            return Err(DeviceInfoError::new(ErrorKind::NotAvailable));
        }
        let symbol = dlsym(library, c"JNI_GetCreatedJavaVMs".as_ptr());
        if symbol.is_null() {
            return Err(DeviceInfoError::new(ErrorKind::NotAvailable));
        }
        let get_created_vms: GetCreatedVms = transmute::<*mut c_void, GetCreatedVms>(symbol);
        let mut vm: *mut sys::JavaVM = null_mut();
        let mut count: jsize = 0;
        if get_created_vms(&mut vm, 1, &mut count) != 0 || count < 1 || vm.is_null() {
            return Err(DeviceInfoError::new(ErrorKind::NotAvailable));
        }
        Ok(vm)
    }
}

/// The application object, as a process-long global reference for `ndk-context`. Null before the
/// runtime has created it.
fn application(env: &mut Env<'_>) -> Result<*mut c_void, DeviceInfoError> {
    let application: JObject<'_> = env
        .call_static_method(
            JNIString::new(ACTIVITY_THREAD_CLASS),
            JNIString::new(CURRENT_APPLICATION),
            jni_sig!("()Landroid/app/Application;"),
            &[],
        )?
        .l()?;
    if application.is_null() {
        return Err(DeviceInfoError::new(ErrorKind::NotAvailable));
    }
    // The global reference lives as long as the process, which is what ndk-context assumes.
    let global = env.new_global_ref(application)?;
    Ok(global.into_raw().cast::<c_void>())
}

/// Gives `ndk-context` the VM and the application, once, so queries reach Java. A context another
/// runtime installed already is adopted untouched.
fn ensure_context() -> Result<(), DeviceInfoError> {
    let mut ready = CONTEXT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *ready {
        return Ok(());
    }
    // `ndk-context` has no try-get and panics while nothing is installed, so a caught probe is
    // the only adoption test; a silent hook keeps the one-time panic out of the log.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let probe = catch_unwind(AssertUnwindSafe(ndk_context::android_context));
    std::panic::set_hook(previous);
    if probe.is_ok() {
        *ready = true;
        return Ok(());
    }
    let vm = find_vm()?;
    // SAFETY: the pointer came from the runtime's own `JNI_GetCreatedJavaVMs`, and the VM
    // lives as long as the process.
    let java_vm = unsafe { JavaVM::from_raw(vm) };
    let application = java_vm.attach_current_thread(application)?;
    // SAFETY: the VM and the application are the process's own, and the probe showed nothing
    // is installed.
    unsafe { ndk_context::initialize_android_context(vm.cast::<c_void>(), application) };
    *ready = true;
    Ok(())
}

/// Runs `work` on this thread, attached to the VM, with the application context at hand.
fn attached<R>(
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> Result<R, DeviceInfoError>,
) -> Result<R, DeviceInfoError> {
    ensure_context()?;
    let context = ndk_context::android_context();
    // SAFETY: the VM pointer is the one this module or the host runtime gave ndk-context, and
    // the VM lives as long as the process.
    let java_vm = unsafe { JavaVM::from_raw(context.vm().cast()) };
    java_vm.attach_current_thread(|env| {
        // SAFETY: ndk-context holds a process-long global reference; the JObject only borrows
        // that pointer and never deletes it.
        let application = unsafe { JObject::from_raw(env, context.context().cast()) };
        work(env, &application)
    })
}

/// Reads a value whose absence is an answer, not an error: a thrown exception is cleared (leaving
/// it pending would break every later JNI call) and read as None.
fn optional<R>(
    env: &mut Env<'_>,
    read: impl FnOnce(&mut Env<'_>) -> Result<R, jni::errors::Error>,
) -> Option<R> {
    match read(env) {
        Ok(value) => Some(value),
        Err(_) => {
            env.exception_clear();
            None
        }
    }
}

/// A Java String's contents as UTF-8.
fn java_string(env: &mut Env<'_>, value: JObject<'_>) -> jni::errors::Result<String> {
    Ok(JString::cast_local(env, value)?.try_to_string(env)?)
}

/// An optional Java String: None for a null reference.
fn optional_string(env: &mut Env<'_>, value: JObject<'_>) -> Option<String> {
    if value.is_null() {
        return None;
    }
    optional(env, |env| java_string(env, value))
}

/// A required Java String: a null read answers the empty string, the way package_info_plus
/// answers a missing versionName, and a thrown exception stays an error.
fn required_string(env: &mut Env<'_>, value: JObject<'_>) -> Result<String, DeviceInfoError> {
    if value.is_null() {
        return Ok(String::new());
    }
    Ok(java_string(env, value)?)
}

fn sdk_int(env: &mut Env<'_>) -> Result<jint, DeviceInfoError> {
    Ok(env
        .get_static_field(
            JNIString::new(VERSION_CLASS),
            JNIString::new(SDK_INT_FIELD),
            jni_sig!("I"),
        )?
        .i()?)
}

fn device_model(env: &mut Env<'_>) -> Result<Model, DeviceInfoError> {
    let model: JObject<'_> = env
        .get_static_field(
            JNIString::new(BUILD_CLASS),
            JNIString::new(MODEL_FIELD),
            jni_sig!("Ljava/lang/String;"),
        )?
        .l()?;
    Ok(Model {
        identifier: optional_string(env, model),
        name: None,
    })
}

fn os_version(env: &mut Env<'_>) -> Result<OsVersion, DeviceInfoError> {
    let sdk = sdk_int(env)?;
    let release: JObject<'_> = env
        .get_static_field(
            JNIString::new(VERSION_CLASS),
            JNIString::new(RELEASE_FIELD),
            jni_sig!("Ljava/lang/String;"),
        )?
        .l()?;
    Ok(assemble_os_version(sdk, optional_string(env, release)))
}

/// versionCode as a decimal string: `getLongVersionCode()` from API 28, the int field below it.
fn version_code(env: &mut Env<'_>, info: &JObject<'_>) -> Result<String, DeviceInfoError> {
    if sdk_int(env)? >= SDK_P {
        Ok(env
            .call_method(info, JNIString::new(LONG_VERSION_CODE), jni_sig!("()J"), &[])?
            .j()?
            .to_string())
    } else {
        Ok(env
            .get_field(info, JNIString::new(VERSION_CODE_FIELD), jni_sig!("I"))?
            .i()?
            .to_string())
    }
}

/// `applicationInfo.loadLabel(packageManager).toString()`; a missing label is the empty string.
fn load_label(
    env: &mut Env<'_>,
    info: &JObject<'_>,
    package_manager: &JObject<'_>,
) -> Result<String, DeviceInfoError> {
    let application_info: JObject<'_> = env
        .get_field(
            info,
            JNIString::new(APPLICATION_INFO_FIELD),
            jni_sig!("Landroid/content/pm/ApplicationInfo;"),
        )?
        .l()?;
    if application_info.is_null() {
        return Ok(String::new());
    }
    let label: JObject<'_> = env
        .call_method(
            &application_info,
            JNIString::new(LOAD_LABEL),
            jni_sig!("(Landroid/content/pm/PackageManager;)Ljava/lang/CharSequence;"),
            &[JValue::Object(package_manager)],
        )?
        .l()?;
    if label.is_null() {
        return Ok(String::new());
    }
    let text: JObject<'_> = env
        .call_method(&label, JNIString::new(TO_STRING), jni_sig!("()Ljava/lang/String;"), &[])?
        .l()?;
    required_string(env, text)
}

/// The store that installed the app: `InstallSourceInfo.getInitiatingPackageName()` from API 30
/// (package_info_plus picks it because a store cannot change it) and
/// `PackageManager.getInstallerPackageName` before it. An absent or unpermitted read is None.
fn installer_package(
    env: &mut Env<'_>,
    sdk: jint,
    package_manager: &JObject<'_>,
    package_name: &str,
) -> Option<String> {
    optional(env, |env| {
        let package: JObject<'_> = env.new_string(package_name)?.into();
        let name: JObject<'_> = if sdk >= SDK_R {
            let source: JObject<'_> = env
                .call_method(
                    package_manager,
                    JNIString::new(GET_INSTALL_SOURCE_INFO),
                    jni_sig!("(Ljava/lang/String;)Landroid/content/pm/InstallSourceInfo;"),
                    &[JValue::Object(&package)],
                )?
                .l()?;
            env.call_method(
                &source,
                JNIString::new(INITIATING_PACKAGE_NAME),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?
        } else {
            env.call_method(
                package_manager,
                JNIString::new(GET_INSTALLER_PACKAGE_NAME),
                jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(&package)],
            )?
            .l()?
        };
        java_string(env, name)
    })
}

/// The first signing certificate's bytes: SigningInfo from API 28 (the first signer, or the
/// first history entry for a single-signer package) and the legacy signatures array before it.
fn certificate_bytes(
    env: &mut Env<'_>,
    sdk: jint,
    package_manager: &JObject<'_>,
    package_name: &str,
) -> Option<Vec<u8>> {
    optional(env, |env| {
        let package: JObject<'_> = env.new_string(package_name)?.into();
        let flags = if sdk >= SDK_P {
            GET_SIGNING_CERTIFICATES
        } else {
            GET_SIGNATURES
        };
        let info: JObject<'_> = env
            .call_method(
                package_manager,
                JNIString::new(GET_PACKAGE_INFO),
                jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/PackageInfo;"),
                &[JValue::Object(&package), JValue::Int(flags)],
            )?
            .l()?;
        let certificates: JObject<'_> = if sdk >= SDK_P {
            let signing_info: JObject<'_> = env
                .get_field(
                    &info,
                    JNIString::new(SIGNING_INFO_FIELD),
                    jni_sig!("Landroid/content/pm/SigningInfo;"),
                )?
                .l()?;
            if signing_info.is_null() {
                return Err(jni::errors::Error::NullPtr(SIGNING_INFO_FIELD));
            }
            let multiple = env
                .call_method(
                    &signing_info,
                    JNIString::new(HAS_MULTIPLE_SIGNERS),
                    jni_sig!("()Z"),
                    &[],
                )?
                .z()?;
            if multiple {
                env.call_method(
                    &signing_info,
                    JNIString::new(APK_CONTENTS_SIGNERS),
                    jni_sig!("()[Landroid/content/pm/Signature;"),
                    &[],
                )?
                .l()?
            } else {
                env.call_method(
                    &signing_info,
                    JNIString::new(SIGNING_CERTIFICATE_HISTORY),
                    jni_sig!("()[Landroid/content/pm/Signature;"),
                    &[],
                )?
                .l()?
            }
        } else {
            env.get_field(
                &info,
                JNIString::new(SIGNATURES_FIELD),
                jni_sig!("()[Landroid/content/pm/Signature;"),
            )?
            .l()?
        };
        if certificates.is_null() {
            return Err(jni::errors::Error::NullPtr(SIGNATURES_FIELD));
        }
        let certificates: SignatureArray<'_> = SignatureArray::cast_local(env, certificates)?;
        // An empty array answers IndexOutOfBounds, which the optional read folds into None.
        let certificate = certificates.get_element(env, 0)?;
        let bytes: JObject<'_> = env
            .call_method(&certificate, JNIString::new(TO_BYTE_ARRAY), jni_sig!("()[B"), &[])?
            .l()?;
        if bytes.is_null() {
            return Err(jni::errors::Error::NullPtr(TO_BYTE_ARRAY));
        }
        let bytes: JByteArray<'_> = JByteArray::cast_local(env, bytes)?;
        env.convert_byte_array(bytes)
    })
}

/// `java.security.MessageDigest`'s SHA-256 of `bytes`, so the crate needs no digest dependency.
fn sha256(env: &mut Env<'_>, bytes: &[u8]) -> jni::errors::Result<Vec<u8>> {
    let algorithm: JObject<'_> = env.new_string(SHA256)?.into();
    let digest: JObject<'_> = env
        .call_static_method(
            JNIString::new(MESSAGE_DIGEST_CLASS),
            JNIString::new(GET_INSTANCE),
            jni_sig!("(Ljava/lang/String;)Ljava/security/MessageDigest;"),
            &[JValue::Object(&algorithm)],
        )?
        .l()?;
    let input: JByteArray<'_> = env.byte_array_from_slice(bytes)?;
    let hashed: JObject<'_> = env
        .call_method(
            &digest,
            JNIString::new(DIGEST),
            jni_sig!("([B)[B"),
            &[JValue::Object(&input)],
        )?
        .l()?;
    if hashed.is_null() {
        return Err(jni::errors::Error::NullPtr(DIGEST));
    }
    let hashed: JByteArray<'_> = JByteArray::cast_local(env, hashed)?;
    env.convert_byte_array(hashed)
}

/// The current signing certificate's SHA-256 as upper-case hex; an unreadable or absent
/// certificate is None, never an error.
fn signing_digest(
    env: &mut Env<'_>,
    sdk: jint,
    package_manager: &JObject<'_>,
    package_name: &str,
) -> Option<String> {
    let bytes = certificate_bytes(env, sdk, package_manager, package_name)?;
    optional(env, |env| Ok(hex_uppercase(&sha256(env, &bytes)?)))
}

fn app_info(env: &mut Env<'_>, context: &JObject<'_>) -> Result<AppInfo, DeviceInfoError> {
    let package_manager: JObject<'_> = env
        .call_method(
            context,
            JNIString::new(GET_PACKAGE_MANAGER),
            jni_sig!("()Landroid/content/pm/PackageManager;"),
            &[],
        )?
        .l()?;
    let package: JObject<'_> = env
        .call_method(
            context,
            JNIString::new(GET_PACKAGE_NAME),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let package_name = required_string(env, package)?;
    let package: JObject<'_> = env.new_string(&package_name)?.into();
    let info: JObject<'_> = env
        .call_method(
            &package_manager,
            JNIString::new(GET_PACKAGE_INFO),
            jni_sig!("(Ljava/lang/String;I)Landroid/content/pm/PackageInfo;"),
            &[JValue::Object(&package), JValue::Int(0)],
        )?
        .l()?;
    let version_name: JObject<'_> = env
        .get_field(&info, JNIString::new(VERSION_NAME_FIELD), jni_sig!("Ljava/lang/String;"))?
        .l()?;
    let version = required_string(env, version_name)?;
    let build_number = version_code(env, &info)?;
    let app_name = load_label(env, &info, &package_manager)?;
    let sdk = sdk_int(env)?;
    let installer_store = installer_package(env, sdk, &package_manager, &package_name);
    let build_signature = signing_digest(env, sdk, &package_manager, &package_name);
    Ok(AppInfo {
        app_name,
        package_name,
        version,
        build_number,
        build_signature,
        installer_store,
    })
}

impl Backend for NativeBackend {
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
        Box::pin(ready(attached(|env, _| device_model(env))))
    }

    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
        Box::pin(ready(attached(|env, _| os_version(env))))
    }

    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
        Box::pin(ready(attached(|env, context| app_info(env, context))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The release keys must stay the exact Java names; a typo compiles and fails only on a device.
    #[test]
    fn the_release_keys_name_the_right_fields() {
        assert_eq!(MODEL_FIELD, "MODEL");
        assert_eq!(SDK_INT_FIELD, "SDK_INT");
        assert_eq!(RELEASE_FIELD, "RELEASE");
        assert_eq!(VERSION_NAME_FIELD, "versionName");
        assert_eq!(VERSION_CODE_FIELD, "versionCode");
        assert_eq!(LONG_VERSION_CODE, "getLongVersionCode");
    }

    /// The InstallSourceInfo accessor names package_info_plus reads the store with.
    #[test]
    fn the_install_source_accessors_follow_the_reference() {
        assert_eq!(GET_INSTALL_SOURCE_INFO, "getInstallSourceInfo");
        assert_eq!(INITIATING_PACKAGE_NAME, "getInitiatingPackageName");
        assert_eq!(GET_INSTALLER_PACKAGE_NAME, "getInstallerPackageName");
    }

    /// Android versions are the SDK level with a zero minor, nsh-core's convention.
    #[test]
    fn sdk_and_release_assemble_the_version() {
        let version = assemble_os_version(34, Some("14".to_string()));
        assert_eq!(version.major, 34);
        assert_eq!(version.minor, 0);
        assert_eq!(version.full.as_deref(), Some("14"));

        let missing = assemble_os_version(-1, None);
        assert_eq!(missing.major, 0);
        assert_eq!(missing.minor, 0);
        assert_eq!(missing.full, None);
    }

    /// package_info_plus formats the certificate digest as upper-case hex.
    #[test]
    fn signature_digests_format_as_upper_case_hex() {
        assert_eq!(hex_uppercase(&[0xde, 0xad, 0xbe, 0xef]), "DEADBEEF");
        assert_eq!(hex_uppercase(&[0x00, 0x0f, 0xa5]), "000FA5");
        assert_eq!(hex_uppercase(&[]), "");
    }

    /// The framework's PackageManager flags and the API levels the reads branch on.
    #[test]
    fn package_flags_and_api_levels_match_the_framework() {
        assert_eq!(GET_SIGNATURES, 0x0000_0040);
        assert_eq!(GET_SIGNING_CERTIFICATES, 0x0800_0000);
        assert_eq!(SDK_P, 28);
        assert_eq!(SDK_R, 30);
    }
}
