//! Every assertion fails a wrong implementation; local runs are forbidden (AGENTS.md), so GitHub CI sees each fail once.

use std::sync::Arc;

use nsh_device_info::{DeviceInfo, DeviceInfoError, ErrorKind, MockBackend};

/// The coded failure a query resolved with; the value it refused is dropped unexamined.
fn refused<T>(answer: Result<T, DeviceInfoError>, what: &str) -> DeviceInfoError {
    answer
        .err()
        .unwrap_or_else(|| panic!("{what} resolved a value instead of its coded failure"))
}

#[test]
fn every_kind_maps_to_its_stable_code() {
    let kinds = [
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
    ];
    for (kind, code, message) in kinds {
        let error = DeviceInfoError::new(kind);
        assert_eq!(error.code(), code, "each kind carries its stable code");
        assert_eq!(error.to_string(), message, "Display shows the kind's own sentence");
    }
}

#[tokio::test]
async fn an_unsupported_target_reports_not_available() {
    let mock = MockBackend::new();
    mock.set_device_model(Err(DeviceInfoError::new(ErrorKind::NotAvailable)));
    mock.set_os_version(Err(DeviceInfoError::new(ErrorKind::NotAvailable)));
    mock.set_app_info(Err(DeviceInfoError::new(ErrorKind::NotAvailable)));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    for error in [
        refused(info.device_model().await, "device_model"),
        refused(info.os_version().await, "os_version"),
        refused(info.app_info().await, "app_info"),
    ] {
        assert!(
            matches!(error.kind(), ErrorKind::NotAvailable),
            "a target with no source reports not available, not a panic or a hang"
        );
        assert_eq!(error.code(), "ERR_DEVICE_INFO_NOT_AVAILABLE");
    }
}

#[tokio::test]
async fn a_platform_failure_carries_the_backend_code_and_status() {
    let mock = MockBackend::new();
    mock.set_device_model(Err(DeviceInfoError::new(ErrorKind::Backend).with_status(-2)));
    mock.set_os_version(Err(DeviceInfoError::new(ErrorKind::Backend).with_status(-2)));
    mock.set_app_info(Err(DeviceInfoError::new(ErrorKind::Backend).with_status(-2)));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    for error in [
        refused(info.device_model().await, "device_model"),
        refused(info.os_version().await, "os_version"),
        refused(info.app_info().await, "app_info"),
    ] {
        assert!(
            matches!(error.kind(), ErrorKind::Backend),
            "a platform failure stays a backend failure"
        );
        assert_eq!(error.code(), "ERR_DEVICE_INFO_BACKEND");
        assert_eq!(error.status(), Some(-2), "the platform's own status survives the trip");
    }
}
