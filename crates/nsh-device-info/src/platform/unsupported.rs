//! A target with no device-info backend: every query reports that nothing is available.

use crate::{
    backend::{Backend, BackendFuture},
    error::{DeviceInfoError, ErrorKind},
    info::{AppInfo, Model, OsVersion},
};

pub(crate) struct NativeBackend;

impl Backend for NativeBackend {
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
        Box::pin(std::future::ready(Err(DeviceInfoError::new(ErrorKind::NotAvailable))))
    }

    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
        Box::pin(std::future::ready(Err(DeviceInfoError::new(ErrorKind::NotAvailable))))
    }

    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
        Box::pin(std::future::ready(Err(DeviceInfoError::new(ErrorKind::NotAvailable))))
    }
}
