//! Linux: the DMI product name is the device model, the device tree's `model` on ARM boards,
//! and `os_info` the OS version.
//!
//! Desktop Linux has no uniform per-app metadata an installed binary can read, so `app_info`
//! answers `ERR_DEVICE_INFO_NOT_AVAILABLE`. A source that exists but cannot be read is the
//! `None` answer, not an error: unprivileged containers are routinely denied DMI reads.

use std::fs;

use crate::{
    backend::{Backend, BackendFuture},
    error::{DeviceInfoError, ErrorKind},
    info::{AppInfo, Model, OsVersion},
};

pub(crate) struct NativeBackend;

/// The desktop's model string, exposed by the firmware through sysfs.
const DMI_PRODUCT_NAME: &str = "/sys/devices/virtual/dmi/id/product_name";

/// ARM boards expose no DMI; the device tree carries the model instead.
const DEVICE_TREE_MODEL: &str = "/proc/device-tree/model";

/// Firmware pads the model string with trailing NUL bytes and a newline, so both are trimmed;
/// content that trims to nothing carries no identifier.
fn identifier_from(content: &str) -> Option<String> {
    let trimmed = content.trim_end_matches(|c| c == '\0' || c.is_whitespace());
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// One short read of a model source: missing, unreadable, or blank is `None`, not an error.
fn read_identifier(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().as_deref().and_then(identifier_from)
}

/// DMI wins when it has a model; Linux reports no marketing name to fill `Model::name`.
fn read_model() -> Model {
    Model {
        identifier: read_identifier(DMI_PRODUCT_NAME)
            .or_else(|| read_identifier(DEVICE_TREE_MODEL)),
        name: None,
    }
}

/// `os_info` never fails: a distro it cannot parse answers `Unknown`, which is `(0, 0, None)`.
fn current_os_version() -> OsVersion {
    OsVersion::from_os_info(&os_info::get())
}

impl Backend for NativeBackend {
    fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
        Box::pin(async { Ok(read_model()) })
    }

    fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
        Box::pin(async { Ok(current_os_version()) })
    }

    fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
        Box::pin(async { Err(DeviceInfoError::new(ErrorKind::NotAvailable)) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sources_are_the_dmi_product_name_and_the_device_tree_model() {
        assert_eq!(DMI_PRODUCT_NAME, "/sys/devices/virtual/dmi/id/product_name");
        assert_eq!(DEVICE_TREE_MODEL, "/proc/device-tree/model");
    }

    #[test]
    fn trailing_nuls_and_whitespace_are_stripped_from_the_identifier() {
        assert_eq!(identifier_from("Mac15,6\n\u{0}").as_deref(), Some("Mac15,6"));
        assert_eq!(identifier_from("  Mac15,6\n\u{0}").as_deref(), Some("  Mac15,6"));
        assert_eq!(
            identifier_from("Raspberry Pi 4 Model B Rev 1.4  \n").as_deref(),
            Some("Raspberry Pi 4 Model B Rev 1.4")
        );
        assert_eq!(
            identifier_from("OptiPlex 7090\u{0}\u{0}\n\u{0}").as_deref(),
            Some("OptiPlex 7090")
        );
    }

    #[test]
    fn blank_content_carries_no_identifier() {
        assert_eq!(identifier_from(""), None);
        assert_eq!(identifier_from("\u{0}\u{0}"), None);
        assert_eq!(identifier_from(" \n\t"), None);
    }

    #[test]
    fn an_unknown_os_info_answer_is_all_zeros() {
        let version = OsVersion::from_os_info(&os_info::Info::unknown());
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 0);
        assert_eq!(version.full, None);
    }
}
