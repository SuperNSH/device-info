//! Every assertion fails a wrong implementation; local runs are forbidden (AGENTS.md), so GitHub CI sees each fail once.

use std::sync::Arc;

use nsh_device_info::{AppInfo, DeviceInfo, DeviceInfoError, MockBackend, Model, OsVersion};

/// The value a query resolved with; only the error's `Display` is assumed, never its `Debug`.
fn answered<T>(answer: Result<T, DeviceInfoError>, what: &str) -> T {
    answer.unwrap_or_else(|error| panic!("{what} resolved an error ({error}), not a value"))
}

#[tokio::test]
async fn each_call_reads_the_backend_again() {
    let mock = MockBackend::new();
    mock.set_device_model(Ok(Model {
        identifier: Some("iPhone16,2".to_owned()),
        name: None,
    }));
    mock.set_os_version(Ok(OsVersion {
        major: 18,
        minor: 1,
        full: Some("18.1".to_owned()),
    }));
    mock.set_app_info(Ok(AppInfo {
        app_name: "Notes".to_owned(),
        package_name: "dev.supernsh.notes".to_owned(),
        version: "1.0.0".to_owned(),
        build_number: "1".to_owned(),
        build_signature: None,
        installer_store: None,
    }));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    let model = answered(info.device_model().await, "device_model");
    assert_eq!(model.identifier.as_deref(), Some("iPhone16,2"));
    let os = answered(info.os_version().await, "os_version");
    assert_eq!(os.major, 18);
    let app = answered(info.app_info().await, "app_info");
    assert_eq!(app.version, "1.0.0");
    assert_eq!(
        (mock.model_reads(), mock.os_version_reads(), mock.app_info_reads()),
        (1, 1, 1),
        "one backend read per call so far"
    );

    // The answers change between calls; only a fresh read on every call can see the new ones.
    mock.set_device_model(Ok(Model {
        identifier: Some("iPhone17,1".to_owned()),
        name: None,
    }));
    mock.set_os_version(Ok(OsVersion {
        major: 19,
        minor: 0,
        full: Some("19.0".to_owned()),
    }));
    mock.set_app_info(Ok(AppInfo {
        app_name: "Notes".to_owned(),
        package_name: "dev.supernsh.notes".to_owned(),
        version: "1.1.0".to_owned(),
        build_number: "2".to_owned(),
        build_signature: None,
        installer_store: None,
    }));

    let model = answered(info.device_model().await, "device_model");
    assert_eq!(model.identifier.as_deref(), Some("iPhone17,1"));
    let os = answered(info.os_version().await, "os_version");
    assert_eq!((os.major, os.minor), (19, 0));
    assert_eq!(os.full.as_deref(), Some("19.0"));
    let app = answered(info.app_info().await, "app_info");
    assert_eq!(app.version, "1.1.0");
    assert_eq!(app.build_number, "2");
    assert_eq!(
        (mock.model_reads(), mock.os_version_reads(), mock.app_info_reads()),
        (2, 2, 2),
        "every call read its backend again"
    );
}

#[tokio::test]
async fn model_keeps_the_raw_identifier() {
    let mock = MockBackend::new();
    mock.set_device_model(Ok(Model {
        identifier: Some("iPhone16,2".to_owned()),
        name: None,
    }));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    let model = answered(info.device_model().await, "device_model");
    assert_eq!(
        model.identifier.as_deref(),
        Some("iPhone16,2"),
        "the raw platform identifier reaches the caller verbatim"
    );
    assert!(model.name.is_none(), "a platform that gives no name gets None");

    mock.set_device_model(Ok(Model {
        identifier: Some("SM-S928B".to_owned()),
        name: Some("Galaxy S24 Ultra".to_owned()),
    }));
    let model = answered(info.device_model().await, "device_model");
    assert_eq!(
        model.identifier.as_deref(),
        Some("SM-S928B"),
        "the identifier is never swapped for a marketing name"
    );
    assert_eq!(
        model.name.as_deref(),
        Some("Galaxy S24 Ultra"),
        "a name the platform itself provides passes through unchanged"
    );
}

#[tokio::test]
async fn partial_data_is_none_not_an_error() {
    let mock = MockBackend::new();
    mock.set_device_model(Ok(Model {
        identifier: None,
        name: None,
    }));
    mock.set_os_version(Ok(OsVersion {
        major: 0,
        minor: 0,
        full: None,
    }));
    mock.set_app_info(Ok(AppInfo {
        app_name: String::new(),
        package_name: String::new(),
        version: String::new(),
        build_number: String::new(),
        build_signature: None,
        installer_store: None,
    }));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    let model = answered(info.device_model().await, "device_model");
    assert!(model.identifier.is_none(), "a missing identifier is None");
    assert!(model.name.is_none(), "a missing name is None");
    let os = answered(info.os_version().await, "os_version");
    assert!(os.full.is_none(), "a missing full version is None");
    let app = answered(info.app_info().await, "app_info");
    assert!(app.build_signature.is_none(), "an unreadable signature is None");
    assert!(app.installer_store.is_none(), "an absent installer store is None");
}

#[tokio::test]
async fn the_facade_passes_through_to_its_backend() {
    let mock = MockBackend::new();
    mock.set_device_model(Ok(Model {
        identifier: Some("Mac15,6".to_owned()),
        name: Some("Mac".to_owned()),
    }));
    mock.set_os_version(Ok(OsVersion {
        major: 15,
        minor: 6,
        full: Some("15.6.1".to_owned()),
    }));
    mock.set_app_info(Ok(AppInfo {
        app_name: "Notes".to_owned(),
        package_name: "dev.supernsh.notes".to_owned(),
        version: "2.3.4".to_owned(),
        build_number: "234".to_owned(),
        build_signature: Some("a1b2c3".to_owned()),
        installer_store: Some("App Store".to_owned()),
    }));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    let model = answered(info.device_model().await, "device_model");
    assert_eq!(model.identifier.as_deref(), Some("Mac15,6"));
    assert_eq!(model.name.as_deref(), Some("Mac"));
    let os = answered(info.os_version().await, "os_version");
    assert_eq!((os.major, os.minor), (15, 6));
    assert_eq!(os.full.as_deref(), Some("15.6.1"));
    let app = answered(info.app_info().await, "app_info");
    assert_eq!(app.app_name, "Notes");
    assert_eq!(app.package_name, "dev.supernsh.notes");
    assert_eq!(app.version, "2.3.4");
    assert_eq!(app.build_number, "234");
    assert_eq!(app.build_signature.as_deref(), Some("a1b2c3"));
    assert_eq!(app.installer_store.as_deref(), Some("App Store"));
    assert_eq!(
        (mock.model_reads(), mock.os_version_reads(), mock.app_info_reads()),
        (1, 1, 1),
        "a pass-through reads each backend query exactly once"
    );
}

#[tokio::test]
async fn unknown_versions_keep_their_string() {
    let mock = MockBackend::new();
    mock.set_os_version(Ok(OsVersion {
        major: 0,
        minor: 0,
        full: Some("nixos 25.05 (Quokka)".to_owned()),
    }));
    let info = DeviceInfo::with_backend(Arc::new(mock.clone()));

    let os = answered(info.os_version().await, "os_version");
    assert_eq!(os.major, 0);
    assert_eq!(os.minor, 0);
    assert_eq!(
        os.full.as_deref(),
        Some("nixos 25.05 (Quokka)"),
        "the platform's own version string is kept verbatim"
    );
}
