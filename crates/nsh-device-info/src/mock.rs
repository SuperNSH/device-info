//! A backend with scripted answers. Tests use it, and so can an app that runs without a platform.

use std::{
    future::ready,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use crate::{
    backend::{Backend, BackendFuture},
    error::DeviceInfoError,
    info::{AppInfo, Model, OsVersion},
};

/// A backend whose answers the test sets and whose reads it counts. Clones share one state.
///
/// The answers are served ready, so no runtime is needed to poll them.
#[derive(Clone)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
}

struct MockState {
    model: Result<Model, DeviceInfoError>,
    os_version: Result<OsVersion, DeviceInfoError>,
    app_info: Result<AppInfo, DeviceInfoError>,
    model_reads: usize,
    os_version_reads: usize,
    app_info_reads: usize,
}

impl Default for MockBackend {
    /// A backend answering empty values; each answer is there to be set.
    fn default() -> Self {
        Self::new()
    }
}

impl MockBackend {
    /// A backend answering `Ok` with missing data until its answers are set.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                model: Ok(Model::default()),
                os_version: Ok(OsVersion::default()),
                app_info: Ok(AppInfo::default()),
                model_reads: 0,
                os_version_reads: 0,
                app_info_reads: 0,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, MockState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets what `device_model` answers.
    pub fn set_device_model(&self, answer: Result<Model, DeviceInfoError>) {
        self.lock().model = answer;
    }

    /// Sets what `os_version` answers.
    pub fn set_os_version(&self, answer: Result<OsVersion, DeviceInfoError>) {
        self.lock().os_version = answer;
    }

    /// Sets what `app_info` answers.
    pub fn set_app_info(&self, answer: Result<AppInfo, DeviceInfoError>) {
        self.lock().app_info = answer;
    }

    /// How many times the device model was read.
    pub fn model_reads(&self) -> usize {
        self.lock().model_reads
    }

    /// How many times the OS version was read.
    pub fn os_version_reads(&self) -> usize {
        self.lock().os_version_reads
    }

    /// How many times the app info was read.
    pub fn app_info_reads(&self) -> usize {
        self.lock().app_info_reads
    }
}

impl Backend for MockBackend {
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
        let answer = {
            let mut state = self.lock();
            state.model_reads += 1;
            state.model.clone()
        };
        Box::pin(ready(answer))
    }

    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
        let answer = {
            let mut state = self.lock();
            state.os_version_reads += 1;
            state.os_version.clone()
        };
        Box::pin(ready(answer))
    }

    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
        let answer = {
            let mut state = self.lock();
            state.app_info_reads += 1;
            state.app_info.clone()
        };
        Box::pin(ready(answer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    #[tokio::test]
    async fn every_read_is_counted_and_clones_share_one_state() {
        let mock = MockBackend::new();
        let clone = mock.clone();
        let hardware = Model {
            identifier: Some("iPhone16,2".to_owned()),
            name: None,
        };
        let backend_failure = DeviceInfoError::new(ErrorKind::Backend).with_status(-2);
        let not_available = DeviceInfoError::new(ErrorKind::NotAvailable);
        clone.set_device_model(Ok(hardware));
        clone.set_os_version(Err(backend_failure));
        clone.set_app_info(Err(not_available));

        let model = mock.device_model().await;
        let identifier = model.ok().and_then(|m| m.identifier);
        assert_eq!(identifier.as_deref(), Some("iPhone16,2"));
        let error = mock.os_version().await.err();
        assert_eq!(error.and_then(|e| e.status()), Some(-2));
        assert!(mock.app_info().await.is_err());

        assert_eq!(clone.model_reads(), 1, "a read through a clone counts once");
        assert_eq!(mock.model_reads(), 1, "clones share one state");
        assert_eq!((mock.os_version_reads(), mock.app_info_reads()), (1, 1));
    }

    #[tokio::test]
    async fn unset_answers_are_missing_data_not_errors() {
        let mock = MockBackend::default();
        assert_eq!(mock.device_model().await, Ok(Model::default()));
        assert_eq!(mock.os_version().await, Ok(OsVersion::default()));
        assert_eq!(mock.app_info().await, Ok(AppInfo::default()));
    }
}
