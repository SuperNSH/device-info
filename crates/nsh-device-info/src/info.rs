//! What a call reports: the device's model, the OS's version and the app's own version.

/// The device's hardware model.
///
/// `identifier` is the raw string the platform reports (`iPhone16,2`, `SM-S928B`, `Mac15,6`, the
/// DMI product name, the UA-CH model). `name` is a marketing name only where the platform itself
/// provides one; this library bundles no mapping table in v0.1. Missing data is `None`, never an
/// error.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Model {
    /// The raw hardware model string, exactly as the platform reports it.
    pub identifier: Option<String>,
    /// The platform's own marketing name for the model, when it has one.
    pub name: Option<String>,
}

/// The operating system's version.
///
/// `major` and `minor` follow nsh-core's convention: iOS reports its own major and minor, Android
/// its `SDK_INT` and 0, the desktops and the web what they parse out of the platform's string.
/// `full` keeps that string whole. An OS version can change between calls (an OS update), so
/// every call reads it again.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OsVersion {
    /// The major version: iOS's major, Android's API level, the desktop's major.
    pub major: u16,
    /// The minor version; 0 where the platform counts in one number (Android).
    pub minor: u16,
    /// The platform's own version string, when it reports one whole.
    pub full: Option<String>,
}

/// The app's own version, as the platform's build metadata reports it.
///
/// `version` and `build_number` follow the platform's names: `CFBundleShortVersionString` /
/// `CFBundleVersion` on Apple, `versionName` / `versionCode` on Android, the exe's `FileVersion`
/// on Windows. A missing individual key is the empty string, as `package_info_plus` behaves; a
/// missing source is a coded error, not empty strings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppInfo {
    /// The app's display name (`CFBundleDisplayName`, the application label, `ProductName`).
    pub app_name: String,
    /// The bundle id or package name (`CFBundleIdentifier`, the Android package, the exe's name).
    pub package_name: String,
    /// The marketing version (`CFBundleShortVersionString`, `versionName`, the `FileVersion`).
    pub version: String,
    /// The build (`CFBundleVersion`, `versionCode`, the `FileVersion`'s last dot-separated part).
    pub build_number: String,
    /// The hex digest of the signing certificate, where the platform exposes one (Android).
    pub build_signature: Option<String>,
    /// The store that installed the app, where the platform names one.
    pub installer_store: Option<String>,
}

/// The three desktop bridges convert `os_info`'s version the same way, so the map lives here once.
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub(crate) fn from_os_info(info: &os_info::Info) -> OsVersion {
    from_os_version(info.version())
}

/// The variant map, split out because `os_info::Info` offers no constructor that sets a version.
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn from_os_version(version: &os_info::Version) -> OsVersion {
    match version {
        // Real OS numbers fit u16; a longer one truncates rather than panicking.
        os_info::Version::Semantic(major, minor, patch) => OsVersion {
            major: *major as u16,
            minor: *minor as u16,
            full: Some(format!("{major}.{minor}.{patch}")),
        },
        os_info::Version::Custom(custom) => OsVersion {
            major: 0,
            minor: 0,
            full: Some(custom.clone()),
        },
        // A rolling release names its version by its date; without one there is no string to keep.
        os_info::Version::Rolling(rolling) => OsVersion {
            major: 0,
            minor: 0,
            full: rolling.clone(),
        },
        os_info::Version::Unknown => OsVersion {
            major: 0,
            minor: 0,
            full: None,
        },
    }
}

// The linux and windows bridges call the method spelling of the same conversion.
#[cfg(any(target_os = "windows", target_os = "linux"))]
impl OsVersion {
    /// The `OsVersion` of an `os_info::Info`; see [`from_os_info`].
    pub(crate) fn from_os_info(info: &os_info::Info) -> OsVersion {
        from_os_info(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_data_starts_empty() {
        // The partial-data answer is built from defaults; no field starts filled.
        assert_eq!(Model::default().identifier, None);
        assert_eq!(Model::default().name, None);
        assert_eq!(OsVersion::default().major, 0);
        assert_eq!(OsVersion::default().minor, 0);
        assert_eq!(OsVersion::default().full, None);
        assert_eq!(AppInfo::default().app_name, "");
        assert_eq!(AppInfo::default().package_name, "");
        assert_eq!(AppInfo::default().version, "");
        assert_eq!(AppInfo::default().build_number, "");
        assert_eq!(AppInfo::default().build_signature, None);
        assert_eq!(AppInfo::default().installer_store, None);
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[test]
    fn from_os_info_maps_every_version_variant() {
        // Exhaustive over os_info's Version; a new variant fails the match in from_os_version.
        let semantic = from_os_version(&os_info::Version::Semantic(15, 2, 7));
        assert_eq!(
            (semantic.major, semantic.minor, semantic.full.as_deref()),
            (15, 2, Some("15.2.7"))
        );

        let unnumbered = from_os_version(&os_info::Version::Semantic(26, 0, 1));
        assert_eq!(
            (unnumbered.major, unnumbered.minor, unnumbered.full.as_deref()),
            (26, 0, Some("26.0.1"))
        );

        let custom = from_os_version(&os_info::Version::Custom("nixos 25.05".to_owned()));
        assert_eq!(
            (custom.major, custom.minor, custom.full.as_deref()),
            (0, 0, Some("nixos 25.05"))
        );

        let dated = from_os_version(&os_info::Version::Rolling(Some("2025.11".to_owned())));
        assert_eq!(
            (dated.major, dated.minor, dated.full.as_deref()),
            (0, 0, Some("2025.11"))
        );

        let undated = from_os_version(&os_info::Version::Rolling(None));
        assert_eq!(
            (undated.major, undated.minor, undated.full.as_deref()),
            (0, 0, None)
        );

        let unknown = from_os_version(&os_info::Version::Unknown);
        assert_eq!(
            (unknown.major, unknown.minor, unknown.full.as_deref()),
            (0, 0, None)
        );
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[test]
    fn from_os_info_reads_an_infos_version() {
        // The pinned entry point: an Info built by os_info itself converts through its version.
        let unknown = from_os_info(&os_info::Info::unknown());
        assert_eq!(unknown.major, 0);
        assert_eq!(unknown.minor, 0);
        assert_eq!(unknown.full, None);
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    #[test]
    fn both_from_os_info_spellings_answer_the_same() {
        let free = from_os_info(&os_info::Info::unknown());
        let method = OsVersion::from_os_info(&os_info::Info::unknown());
        assert_eq!(method, free);
    }
}
