//! The errors a device-info call reports. Each has a stable code; a message says what happened.

use std::fmt;

/// What went wrong, as a stable code; see [`DeviceInfoError::code`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The platform has no source for this query (the web's app info, desktop Linux's app info,
    /// a target with no backend).
    NotAvailable,
    /// The platform failed; `status` holds its own code where it gives one.
    Backend,
}

/// A failed device-info call.
///
/// `Display` shows the kind's message. Neither it nor the code holds reason text or any user data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", .kind.message())]
pub struct DeviceInfoError {
    kind: ErrorKind,
    status: Option<i64>,
}

impl DeviceInfoError {
    /// An error of `kind`, with no platform status.
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, status: None }
    }

    /// The same error, carrying the platform's own status code.
    #[must_use]
    pub fn with_status(mut self, status: i64) -> Self {
        self.status = Some(status);
        self
    }

    /// What went wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The platform's status code, when the platform gave one.
    pub fn status(&self) -> Option<i64> {
        self.status
    }

    /// The stable code, such as `ERR_DEVICE_INFO_NOT_AVAILABLE`. Apps match on this.
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }
}

impl ErrorKind {
    /// The stable code of this kind.
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::NotAvailable => "ERR_DEVICE_INFO_NOT_AVAILABLE",
            ErrorKind::Backend => "ERR_DEVICE_INFO_BACKEND",
        }
    }

    /// What happened, in one sentence.
    pub fn message(self) -> &'static str {
        match self {
            ErrorKind::NotAvailable => "Device information is not available on this platform.",
            ErrorKind::Backend => "The platform's device information service failed.",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_maps_to_its_stable_code() {
        // Exhaustive: adding a kind without a code fails here.
        for (kind, code) in [
            (ErrorKind::NotAvailable, "ERR_DEVICE_INFO_NOT_AVAILABLE"),
            (ErrorKind::Backend, "ERR_DEVICE_INFO_BACKEND"),
        ] {
            assert_eq!(kind.code(), code);
            assert_eq!(DeviceInfoError::new(kind).code(), code);
            assert_eq!(kind.to_string(), code);
        }
    }

    #[test]
    fn every_kind_maps_to_its_message() {
        // Exhaustive: adding a kind without a message fails here. Display shows only the message.
        for (kind, message) in [
            (ErrorKind::NotAvailable, "Device information is not available on this platform."),
            (ErrorKind::Backend, "The platform's device information service failed."),
        ] {
            assert_eq!(kind.message(), message);
            assert_eq!(DeviceInfoError::new(kind).to_string(), message);
        }
    }

    #[test]
    fn a_status_travels_with_the_error() {
        assert_eq!(DeviceInfoError::new(ErrorKind::Backend).status(), None);
        let error = DeviceInfoError::new(ErrorKind::Backend).with_status(-2);
        assert_eq!(error.kind(), ErrorKind::Backend);
        assert_eq!(error.status(), Some(-2));
    }
}
