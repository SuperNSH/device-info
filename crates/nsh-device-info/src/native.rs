//! The device info as TSX sees it: `NativeDeviceInfo`, a class whose async members reject with a
//! `code` on failure. Results are plain values; the reads stay on the Rust side.

use nsh_core::prelude::*;

use crate::{AppInfo, DeviceInfo, DeviceInfoError, Model, OsVersion};

/// The model as TSX receives it: `{ identifier, name }`, each `null` where nothing is reported.
#[derive(JsObject, Clone, Debug)]
pub struct NativeModel {
    /// The raw hardware model string, when the platform reports one.
    pub identifier: Option<String>,
    /// The platform's own marketing name, when it has one.
    pub name: Option<String>,
}

/// The OS version as TSX receives it: `{ major, minor, full }`.
#[derive(JsObject, Clone, Debug)]
pub struct NativeOsVersion {
    /// The major version: iOS's major, Android's API level, the desktop's major.
    pub major: u16,
    /// The minor version; 0 where the platform counts in one number.
    pub minor: u16,
    /// The platform's own version string, when it reports one whole.
    pub full: Option<String>,
}

/// The app info as TSX receives it: `{ appName, packageName, version, buildNumber,
/// buildSignature, installerStore }`.
#[derive(JsObject, Clone, Debug)]
pub struct NativeAppInfo {
    /// The app's display name.
    pub app_name: String,
    /// The bundle id or package name.
    pub package_name: String,
    /// The marketing version.
    pub version: String,
    /// The build.
    pub build_number: String,
    /// The hex digest of the signing certificate, where the platform exposes one.
    pub build_signature: Option<String>,
    /// The store that installed the app, where the platform names one.
    pub installer_store: Option<String>,
}

/// A device-info handle. Create one and keep it; every member reads the platform again.
#[class]
pub struct NativeDeviceInfo {
    info: DeviceInfo,
}

#[class]
impl NativeDeviceInfo {
    /// The platform's device information.
    pub fn new() -> NativeDeviceInfo {
        NativeDeviceInfo {
            info: DeviceInfo::new(),
        }
    }

    /// The device's model, as the platform reports it.
    pub async fn device_model(this: Instance<Self>) -> Result<NativeModel, JsError> {
        let info = this.with(|native| native.info.clone());
        let model = info.device_model().await.map_err(into_js)?;
        Ok(NativeModel::from(model))
    }

    /// The OS's version, read again on every call.
    pub async fn os_version(this: Instance<Self>) -> Result<NativeOsVersion, JsError> {
        let info = this.with(|native| native.info.clone());
        let version = info.os_version().await.map_err(into_js)?;
        Ok(NativeOsVersion::from(version))
    }

    /// The app's own name and version, from the build metadata.
    pub async fn app_info(this: Instance<Self>) -> Result<NativeAppInfo, JsError> {
        let info = this.with(|native| native.info.clone());
        let app = info.app_info().await.map_err(into_js)?;
        Ok(NativeAppInfo::from(app))
    }
}

impl Default for NativeDeviceInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Model> for NativeModel {
    fn from(model: Model) -> Self {
        Self {
            identifier: model.identifier,
            name: model.name,
        }
    }
}

impl From<OsVersion> for NativeOsVersion {
    fn from(version: OsVersion) -> Self {
        Self {
            major: version.major,
            minor: version.minor,
            full: version.full,
        }
    }
}

impl From<AppInfo> for NativeAppInfo {
    fn from(app: AppInfo) -> Self {
        Self {
            app_name: app.app_name,
            package_name: app.package_name,
            version: app.version,
            build_number: app.build_number,
            build_signature: app.build_signature,
            installer_store: app.installer_store,
        }
    }
}

/// The error a rejection carries: its message, and the `code` apps match on.
fn into_js(error: DeviceInfoError) -> JsError {
    JsError::Error(error.kind().message().to_owned()).with_code(error.code())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn the_js_shapes_keep_every_field() {
        let hardware = Model {
            identifier: Some("iPhone16,2".to_owned()),
            name: Some("iPhone".to_owned()),
        };
        let model = NativeModel::from(hardware);
        assert_eq!(model.identifier.as_deref(), Some("iPhone16,2"));
        assert_eq!(model.name.as_deref(), Some("iPhone"));

        let version = NativeOsVersion::from(OsVersion {
            major: 18,
            minor: 1,
            full: Some("18.1".to_owned()),
        });
        assert_eq!(version.major, 18);
        assert_eq!(version.minor, 1);
        assert_eq!(version.full.as_deref(), Some("18.1"));

        let store = AppInfo {
            app_name: "Notes".to_owned(),
            package_name: "dev.supernsh.notes".to_owned(),
            version: "2.3.4".to_owned(),
            build_number: "234".to_owned(),
            build_signature: Some("a1b2c3".to_owned()),
            installer_store: Some("App Store".to_owned()),
        };
        let app = NativeAppInfo::from(store);
        assert_eq!(app.app_name, "Notes");
        assert_eq!(app.package_name, "dev.supernsh.notes");
        assert_eq!(app.version, "2.3.4");
        assert_eq!(app.build_number, "234");
        assert_eq!(app.build_signature.as_deref(), Some("a1b2c3"));
        assert_eq!(app.installer_store.as_deref(), Some("App Store"));
    }

    #[test]
    fn a_rejection_carries_the_code_and_the_message() {
        for (kind, code, message) in [
            (
                ErrorKind::NotAvailable,
                "ERR_DEVICE_INFO_NOT_AVAILABLE",
                "Device information is not available on this platform.",
            ),
            (
                ErrorKind::Backend,
                "ERR_DEVICE_INFO_BACKEND",
                "The platform's device information service failed.",
            ),
        ] {
            let js = into_js(DeviceInfoError::new(kind));
            assert_eq!(js.message(), message);
            assert_eq!(js.code(), Some(code));
        }
    }
}
