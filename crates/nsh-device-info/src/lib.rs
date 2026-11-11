//! Device model, OS version and app version for Native Shell apps.
//!
//! [`DeviceInfo`] reads what the platform itself reports: the raw model identifier, the OS's own
//! version numbers, the app's build metadata. Each call is one quick read that resolves exactly
//! once, with the values or a [`DeviceInfoError`] carrying a stable code; nothing is cached, so
//! an OS update shows on the next call.

mod backend;
mod error;
mod info;
mod mock;
mod native;
mod platform;

use std::sync::Arc;

use nsh_core::prelude::*;

pub use backend::{Backend, BackendFuture};
pub use error::{DeviceInfoError, ErrorKind};
pub use info::{AppInfo, Model, OsVersion};
pub use mock::MockBackend;
pub use native::{NativeAppInfo, NativeDeviceInfo, NativeModel, NativeOsVersion};

/// The device information of the app's platform.
///
/// Clones share one backend. [`DeviceInfo::new`] uses the platform's own; [`DeviceInfo::with_backend`]
/// runs any other, which the tests and the mock use. Every query reads the platform again — an
/// OS version can change between calls.
#[derive(Clone)]
pub struct DeviceInfo {
    backend: Arc<dyn Backend>,
}

impl DeviceInfo {
    /// The platform's device information.
    pub fn new() -> Self {
        Self {
            backend: platform::default_backend(),
        }
    }

    /// Device information through `backend`. Tests and mocks use this.
    pub fn with_backend(backend: Arc<dyn Backend>) -> Self {
        Self { backend }
    }

    /// The operating system of this build, from the target: `platform.os`'s six values.
    pub fn os(&self) -> nsh_core::platform::Os {
        current_os()
    }

    /// The device's hardware model, as the platform reports it.
    pub async fn device_model(&self) -> Result<Model, DeviceInfoError> {
        self.backend.device_model().await
    }

    /// The OS's version, read again on every call.
    pub async fn os_version(&self) -> Result<OsVersion, DeviceInfoError> {
        self.backend.os_version().await
    }

    /// The app's own name and version, from the build metadata.
    pub async fn app_info(&self) -> Result<AppInfo, DeviceInfoError> {
        self.backend.app_info().await
    }
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// The OS of the target, without a runtime: the same answer `platform.os` gives.
fn current_os() -> nsh_core::platform::Os {
    // A target outside the six reports the headless default, as nsh-core's Os::default does.
    match () {
        _ if cfg!(target_os = "ios") => nsh_core::platform::Os::Ios,
        _ if cfg!(target_os = "android") => nsh_core::platform::Os::Android,
        _ if cfg!(target_os = "macos") => nsh_core::platform::Os::MacOs,
        _ if cfg!(target_os = "windows") => nsh_core::platform::Os::Windows,
        _ if cfg!(target_os = "linux") => nsh_core::platform::Os::Linux,
        _ if cfg!(target_family = "wasm") => nsh_core::platform::Os::Web,
        _ => nsh_core::platform::Os::Ios,
    }
}

/// The crate's UI set: what TSX imports from `nsh-device-info/native`.
#[native_module("nsh-device-info/native")]
fn native(registry: &mut Registry) {
    registry.class::<NativeDeviceInfo>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsh_core::platform::Os;

    #[test]
    fn os_follows_the_build_target() {
        // Exhaustive: every Os carries platform.os's own name.
        for (os, name) in [
            (Os::Ios, "ios"),
            (Os::Android, "android"),
            (Os::MacOs, "macos"),
            (Os::Windows, "windows"),
            (Os::Linux, "linux"),
            (Os::Web, "web"),
        ] {
            assert_eq!(os.name(), name);
        }
        let expected = match () {
            _ if cfg!(target_os = "ios") => Os::Ios,
            _ if cfg!(target_os = "android") => Os::Android,
            _ if cfg!(target_os = "macos") => Os::MacOs,
            _ if cfg!(target_os = "windows") => Os::Windows,
            _ if cfg!(target_os = "linux") => Os::Linux,
            _ if cfg!(target_family = "wasm") => Os::Web,
            _ => Os::Ios,
        };
        assert_eq!(DeviceInfo::new().os(), expected);
    }
}
