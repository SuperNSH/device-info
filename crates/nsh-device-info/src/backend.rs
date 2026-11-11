//! The seam between `DeviceInfo` and a platform: three quick reads, each answered once.

use std::{future::Future, pin::Pin};

use crate::{
    error::DeviceInfoError,
    info::{AppInfo, Model, OsVersion},
};

/// A future a backend returns. Each call's future borrows the backend.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// What a platform provides: the device's model, the OS's version and the app's own version.
/// Each call is a quick read that resolves exactly once — a value, or a coded failure.
pub trait Backend: Send + Sync {
    /// The device's hardware model, as the platform reports it.
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>>;

    /// The OS's version, read again on every call.
    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>>;

    /// The app's own name and version, from the build metadata.
    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>>;
}
