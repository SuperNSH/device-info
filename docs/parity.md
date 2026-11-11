# Parity with `device_info_plus`, `package_info_plus` and `react-native-device-info`

| Capability | Status | Evidence |
|---|---|---|
| Crate builds and CI passes | Unverified | every source is in the checkout — the crate, its platform bridges and the TypeScript package — and the gates are wired in `.github/workflows/ci.yml`; no run has passed, so this stays unverified until the next CI run on `master` |
| Device info API | Done | `the_facade_passes_through_to_its_backend`, `each_call_reads_the_backend_again`, `every_kind_maps_to_its_stable_code` |
| Platform implementations | Partial | Linux and the shared helpers run in CI; the Apple, Android, Windows and web bridges are Platform only below |
| Device tests | Done | eight of the ten canonical tests in `crates/nsh-device-info/tests/facade.rs` and `tests/errors.rs`, the same eight sentence names in `packages/nsh-device-info/tests/`; #3 `a_simulator_reports_a_simulator_model` and #4 `the_release_keys_name_the_right_fields` live inline in the platform files, and the TS side has no #4 test |
| Status page | Done | `site/index.html`, published by `.github/workflows/pages.yml` |

Each row below names one public API of the references — `device_info_plus` 13.3.0, `package_info_plus` 10.2.2 and `react-native-device-info` 15.0.2, read from `.refs/` on 2026-11-09 — what answers it here, and the test that covers it. This library answers three questions: which device (`Model`), which OS (`OsVersion`), which app (`AppInfo`).

Rows describe the code in this repo, not intentions. The named test is each row's evidence; the CI run on `master` is the proof it passes. Eight canonical tests live in `crates/nsh-device-info/tests/facade.rs` and `tests/errors.rs`, and the same eight sentence names in `packages/nsh-device-info/tests/facade.test.ts` and `errors.test.ts`; #3 `a_simulator_reports_a_simulator_model` and #4 `the_release_keys_name_the_right_fields` live inline in the platform files, and the TS side carries no #4 test — its surface has no platform key constants to assert. Inline helper tests are named with their file in the row, and a citation names only a test that exists in this checkout. A react-native-device-info row names a sync twin with its async twin — the same capability exposed twice. The native module's hidden methods (`getPowerState`/`getPowerStateSync`, `getSupportedAbis`/`getSupported32BitAbis`/`getSupported64BitAbis` and their Sync twins, `getSystemManufacturer`/`getSystemManufacturerSync`) are the same capabilities as the members above and share their rows. All answers here are async futures or promises; this library has no sync twins.

Status words:

- **Done**: works as the reference does.
- **Partial**: works, with the limit named in the row.
- **Platform only**: needs hardware or a browser no CI runner has; the named test covers the helper or the shared shape.
- **Not planned**: beyond model / OS version / app version; the row names what the app uses instead.

No reference API is accepted-and-ignored: every name either maps to the three answers or is not planned.

## `DeviceInfoPlugin`

The reference caches each platform answer ("This information does not change from call to call. Cache it."). This library never caches: every call reads the backend again, because an OS version can change between calls.

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `DeviceInfoPlugin()` | `DeviceInfo::new()` | `DeviceInfo` | Done. Cheap to construct; picks the target's default backend. `DeviceInfo::with_backend(Arc<dyn Backend>)` swaps it in tests | `the_facade_passes_through_to_its_backend` |
| `deviceInfo` | `DeviceInfo::device_model()` / `os_version()` / `app_info()` | `DeviceInfo.deviceModel()` / `osVersion()` / `appInfo()` | Done. Three typed answers instead of one platform-shaped map, re-read on every call | `the_facade_passes_through_to_its_backend`; `each_call_reads_the_backend_again` |
| `androidInfo` | the same three, Android backend | the same three | Done. `Build.MODEL`, `Build.VERSION` and the package manager over JNI. Needs a device; the field rows below carry the limits | `the_release_keys_name_the_right_fields` (inline in `android.rs`) |
| `iosInfo` | the same three, Apple backend | the same three | Done. utsname and `UIDevice` for model and version, `NSBundle` for the app. The simulator case is inline-tested | `a_simulator_reports_a_simulator_model` (inline in `apple.rs`) |
| `linuxInfo` | the same three, Linux backend | the same three | Done. os_info for the version; the model is the DMI product name, falling back to the device tree's `model` on ARM boards. App info is the Linux gap — see `fromPlatform` below | `the_sources_are_the_dmi_product_name_and_the_device_tree_model`, `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`) |
| `macOsInfo` | the same three, Apple backend | the same three | Done. sysctl `hw.model` for the identifier, os_info for the version, `NSBundle` for the app | `the_release_keys_name_the_right_fields` (inline in `apple.rs`) |
| `windowsInfo` | the same three, Windows backend | the same three | Done. The registry's firmware product name for the machine, os_info for the version, the exe's version resource (`GetFileVersionInfoW`/`VerQueryValueW`) for the app and build | `the_release_keys_name_the_right_fields` (inline in `windows.rs`) |
| `webBrowserInfo` | the same three, web backend | the same three | Partial. The model is UA-CH-only: served where the browser gives it, null where it does not. The OS version is UA-CH where served and falls back to a best-effort `navigator.userAgent` parse when the browser withholds UA-CH; app info is `ERR_DEVICE_INFO_NOT_AVAILABLE` | `model_extraction_keeps_the_raw_string`; `platform_version_reduced_or_absent_is_zeros` (inline in `web.rs`) |
| `setMockInitialValues` (testing) | `MockBackend` + `DeviceInfo::with_backend` | `memoryBackend` (`src/memory.ts`) | Done. Tests swap the backend instead of seeding a cache, so re-reads stay observable and counted | `each_call_reads_the_backend_again` |

## `BaseDeviceInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `data`, `toMap()`, `fromMap()` on every model | not in the API | not in the API | Not planned. The typed fields are the API; no raw platform map is passed through in either direction. An app needing a field this library does not carry reads the platform itself | none |

## `AndroidDeviceInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `model` | `Model.identifier` | `Model.identifier` | Platform only. Needs an Android device; the JNI read keeps the raw `Build.MODEL` string ("SM-S928B") | `the_release_keys_name_the_right_fields` (inline in `android.rs`) |
| `version.sdkInt` | `OsVersion.major` | `OsVersion.major` | Platform only. The API level, as `Build.VERSION.SDK_INT`; `minor` is 0 | `sdk_and_release_assemble_the_version`; `the_release_keys_name_the_right_fields` (inline in `android.rs`) |
| `version.release` | `OsVersion.full` | `OsVersion.full` | Platform only. `Build.VERSION.RELEASE` verbatim | `sdk_and_release_assemble_the_version`; `the_release_keys_name_the_right_fields` (inline in `android.rs`) |
| `version.baseOS` | not in the API | not in the API | Not planned. Beyond model / OS version / app version; no NSH API serves it | none |
| `version.codename` | not in the API | not in the API | Not planned. Same scope line as `baseOS` | none |
| `version.incremental` | not in the API | not in the API | Not planned. Same scope line as `baseOS` | none |
| `version.previewSdkInt` | not in the API | not in the API | Not planned. Same scope line as `baseOS` | none |
| `version.securityPatch` | not in the API | not in the API | Not planned. Same scope line as `baseOS` | none |
| `board` | not in the API | not in the API | Not planned. `android.os.Build` fields beyond MODEL and VERSION | none |
| `bootloader` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `brand` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `device` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `display` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `fingerprint` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `hardware` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `host` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `id` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `manufacturer` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `product` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `name` | not in the API | not in the API | Not planned. The user's device name; privacy data, not model data | none |
| `supported32BitAbis` | not in the API | not in the API | Not planned. ABI inventory; the build target knows the ABI at compile time | none |
| `supported64BitAbis` | not in the API | not in the API | Not planned. Same scope line as `supported32BitAbis` | none |
| `supportedAbis` | not in the API | not in the API | Not planned. Same scope line as `supported32BitAbis` | none |
| `tags` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `time` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `type` | not in the API | not in the API | Not planned. Same scope line as `board` | none |
| `isPhysicalDevice` | not in the API | not in the API | Not planned. No boolean: the raw identifier is the truth, and a simulator reports a simulator model | `model_keeps_the_raw_identifier`; `a_simulator_reports_a_simulator_model` |
| `systemFeatures` | not in the API | not in the API | Not planned. Feature discovery is the package manager's, not this library's | none |
| `isLowRamDevice` | not in the API | not in the API | Not planned. Memory class; no NSH API serves it | none |
| `physicalRamSize` | not in the API | not in the API | Not planned. Memory inventory | none |
| `availableRamSize` | not in the API | not in the API | Not planned. Memory inventory | none |
| `freeDiskSize` | not in the API | not in the API | Not planned. Storage; the platform's own file-system APIs | none |
| `totalDiskSize` | not in the API | not in the API | Not planned. Storage | none |

## `IosDeviceInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `name` | not in the API | not in the API | Not planned. The user-assigned device name is privacy data; no NSH API serves it | none |
| `systemName` | `DeviceInfo::os()` -> `Os::Ios` | `DeviceInfo.os` | Partial. A typed `Os`, compile-time, not the "iOS" display string; the app formats its own label | `the_facade_passes_through_to_its_backend` |
| `systemVersion` | `OsVersion` | `OsVersion` | Platform only. Needs an iOS device; `major`/`minor` parsed from `UIDevice`'s `systemVersion`, `full` "major.minor" with the patch appended only when nonzero, an unparseable string is `ERR_DEVICE_INFO_BACKEND` | `operating_system_version_assembles_major_minor_patch` (inline in `apple.rs`) |
| `model` | `Model.name` | `Model.name` | Partial. `Model.name` stays null on Apple in v0.1: UIKit's `model` only says "iPhone", a class rather than a name this library reports | `partial_data_is_none_not_an_error` |
| `modelName` | `Model.name` | `Model.name` | Partial. The reference maps identifiers to commercial names from a bundled table; v0.1 bundles no table and Apple reports no name of its own, so this is null | `partial_data_is_none_not_an_error` |
| `localizedModel` | `Model.name` | `Model.name` | Partial. Null in v0.1 for the same reason as `modelName`: the localized class string ("iPhone") is not reported as a name | `partial_data_is_none_not_an_error` |
| `identifierForVendor` | not in the API | not in the API | Not planned. A tracking identifier; the app uses its own account identity | none |
| `isPhysicalDevice` | not in the API | not in the API | Not planned. No boolean; a simulator reports its own machine string, which the inline test pins | `a_simulator_reports_a_simulator_model` |
| `isiOSAppOnMac` | not in the API | not in the API | Not planned. Which host an iOS app runs on is out of scope | none |
| `isiOSAppOnVision` | not in the API | not in the API | Not planned. Same scope line as `isiOSAppOnMac` | none |
| `utsname.machine` | `Model.identifier` | `Model.identifier` | Platform only. Needs Apple hardware; the raw "iPhone16,2", and a simulator reports the simulator's machine string, not a fake iPhone | `model_keeps_the_raw_identifier`; `a_simulator_reports_a_simulator_model` (inline in `apple.rs`) |
| `utsname.sysname` | not in the API | not in the API | Not planned. Kernel strings; `DeviceInfo.os` already names the system | none |
| `utsname.nodename` | not in the API | not in the API | Not planned. A machine name is an identifier; out of scope | none |
| `utsname.release` | not in the API | not in the API | Not planned. The kernel build; `OsVersion` carries the OS version | none |
| `utsname.version` | not in the API | not in the API | Not planned. Same scope line as `utsname.release` | none |
| `physicalRamSize` | not in the API | not in the API | Not planned. Memory inventory | none |
| `availableRamSize` | not in the API | not in the API | Not planned. Memory inventory | none |
| `freeDiskSize` | not in the API | not in the API | Not planned. Storage | none |
| `totalDiskSize` | not in the API | not in the API | Not planned. Storage | none |

## `LinuxDeviceInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `name` | not in the API | not in the API | Not planned. Distro identity strings; `DeviceInfo.os` already says Linux | none |
| `version` | `OsVersion` | `OsVersion` | Done. os_info's parsed version: semantic distro versions land in `major`/`minor`/`full`, and a distro os_info cannot parse answers (0, 0, null) | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`) |
| `id` | not in the API | not in the API | Not planned. `/etc/os-release` fields beyond the version | none |
| `idLike` | not in the API | not in the API | Not planned. Same scope line as `id` | none |
| `versionCodename` | not in the API | not in the API | Not planned. Same scope line as `id` | none |
| `versionId` | `OsVersion.full` | `OsVersion.full` | Done. The distro version string, through the same conversion as `version` | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`) |
| `prettyName` | not in the API | not in the API | Not planned. Display string; the app builds its own from `Os` and `OsVersion` | none |
| `buildId` | not in the API | not in the API | Not planned. Same scope line as `id` | none |
| `variant` | not in the API | not in the API | Not planned. Same scope line as `id` | none |
| `variantId` | not in the API | not in the API | Not planned. Same scope line as `id` | none |
| `machineId` | not in the API | not in the API | Not planned. A machine identifier is tracking data; the app generates its own | none |
| not in `LinuxDeviceInfo`: hardware model | `Model.identifier` (DMI product name, device-tree `model` fallback) | `Model.identifier` | Partial. The reference has no model on Linux; this reads the DMI product name and falls back to the device tree's `model` on ARM boards, and a machine with neither (a container) reports null, not an error | `the_sources_are_the_dmi_product_name_and_the_device_tree_model` (inline in `linux.rs`) |

## `MacOsDeviceInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `computerName` | not in the API | not in the API | Not planned. The user's machine name; privacy data | none |
| `hostName` | not in the API | not in the API | Not planned. Same scope line as `computerName` | none |
| `arch` | not in the API | not in the API | Not planned. The build target knows the arch at compile time; `cfg` serves it | none |
| `model` | `Model.identifier` | `Model.identifier` | Platform only. Needs macOS; the raw sysctl `hw.model` string ("Mac15,6") | `the_sysctl_name_resolves_to_the_hw_model_mib` (inline in `apple.rs`) |
| `modelName` | `Model.name` | `Model.name` | Partial. The reference's commercial names come from a bundled table; v0.1 bundles none and macOS provides no name of its own, so this is null | `partial_data_is_none_not_an_error` |
| `kernelVersion` | not in the API | not in the API | Not planned. Kernel strings; `OsVersion` carries the OS version | none |
| `osRelease` | not in the API | not in the API | Not planned. Same scope line as `kernelVersion` | none |
| `majorVersion` | `OsVersion.major` | `OsVersion.major` | Done. The shared os_info conversion, inline-tested | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`, `windows.rs`) |
| `minorVersion` | `OsVersion.minor` | `OsVersion.minor` | Done. Same conversion as `majorVersion` | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`, `windows.rs`) |
| `patchVersion` | folded into `OsVersion.full` | folded into `OsVersion.full` | Partial. The patch is kept in the full "major.minor.patch" string, not a separate field | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`, `windows.rs`) |
| `activeCPUs` | not in the API | not in the API | Not planned. Hardware inventory; `std::thread::available_parallelism` serves it | none |
| `memorySize` | not in the API | not in the API | Not planned. Hardware inventory | none |
| `cpuFrequency` | not in the API | not in the API | Not planned. Hardware inventory | none |
| `systemGUID` | not in the API | not in the API | Not planned. A machine identifier is tracking data | none |

## `WebBrowserInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `browserName` | not in the API | not in the API | Not planned. UA sniffing for a browser enum; the app parses `navigator.userAgent` itself if it must | none |
| `appCodeName` | not in the API | not in the API | Not planned. Legacy navigator strings, documented unreliable in the reference itself | none |
| `appName` | not in the API | not in the API | Not planned. Same scope line as `appCodeName` | none |
| `appVersion` | not in the API | not in the API | Not planned. Same scope line as `appCodeName` | none |
| `deviceMemory` | not in the API | not in the API | Not planned. The app reads `navigator.deviceMemory` directly | none |
| `language` | not in the API | not in the API | Not planned. Locale, not device info; `navigator.language` | none |
| `languages` | not in the API | not in the API | Not planned. Locale; `navigator.languages` | none |
| `platform` | not in the API | not in the API | Not planned. `navigator.platform` is frozen or reduced by browsers; `DeviceInfo.os` reports Web | none |
| `product` | not in the API | not in the API | Not planned. Same scope line as `appCodeName` | none |
| `productSub` | not in the API | not in the API | Not planned. Same scope line as `appCodeName` | none |
| `userAgent` | not in the API | not in the API | Not planned. The app reads `navigator.userAgent`; the model here comes from UA-CH only, and the OS version falls back to a user-agent parse when the browser withholds UA-CH | none |
| `vendor` | not in the API | not in the API | Not planned. Same scope line as `appCodeName` | none |
| `vendorSub` | not in the API | not in the API | Not planned. Same scope line as `appCodeName` | none |
| `hardwareConcurrency` | not in the API | not in the API | Not planned. The app reads `navigator.hardwareConcurrency` directly | none |
| `maxTouchPoints` | not in the API | not in the API | Not planned. The app reads `navigator.maxTouchPoints` directly | none |
| not in `WebBrowserInfo`: UA-CH `model` | `Model.identifier` | `Model.identifier` | Platform only. Needs a browser; `getHighEntropyValues(["model"])` where served — UA-CH carries no marketing name, so `name` is null — and absent or reduced to empty is null | `model_extraction_keeps_the_raw_string` (inline in `web.rs`) |
| not in `WebBrowserInfo`: UA-CH `platformVersion` | `OsVersion` | `OsVersion` | Platform only. Needs a browser; `platformVersion` parsed where served, reduced or absent gives (0, 0, null), an unparseable string is kept in `full`; when the browser withholds UA-CH the OS version is parsed best-effort out of `navigator.userAgent`, and an agent with no OS token gives (0, 0, null) | `platform_version_reduced_or_absent_is_zeros`; `platform_version_keeps_an_unparseable_string`; `user_agent_tokens_parse_per_platform` (inline in `web.rs`) |

## `WindowsDeviceInfo`

| `device_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `computerName` | not in the API | not in the API | Not planned. The machine's DNS name; privacy data | none |
| `numberOfCores` | not in the API | not in the API | Not planned. Hardware inventory; `std::thread::available_parallelism` serves it | none |
| `systemMemoryInMegabytes` | not in the API | not in the API | Not planned. Hardware inventory | none |
| `userName` | not in the API | not in the API | Not planned. The logged-in user; privacy data | none |
| `majorVersion` | `OsVersion.major` | `OsVersion.major` | Done. The shared os_info conversion, inline-tested | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`, `windows.rs`) |
| `minorVersion` | `OsVersion.minor` | `OsVersion.minor` | Done. Same conversion as `majorVersion` | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `linux.rs`, `windows.rs`) |
| `buildNumber` | folded into `OsVersion.full` | folded into `OsVersion.full` | Partial. The build rides in the full "major.minor.build" string, not a separate field | `unknown_versions_keep_their_string` (`tests/facade.rs`); `an_unknown_os_info_answer_is_all_zeros` (inline in `windows.rs`) |
| `platformId` | not in the API | not in the API | Not planned. Always `VER_PLATFORM_WIN32_NT` on every supported Windows | none |
| `csdVersion` | not in the API | not in the API | Not planned. Service-pack strings on a dead Windows line | none |
| `servicePackMajor` | not in the API | not in the API | Not planned. Same scope line as `csdVersion` | none |
| `servicePackMinor` | not in the API | not in the API | Not planned. Same scope line as `csdVersion` | none |
| `suitMask` | not in the API | not in the API | Not planned. Product suites | none |
| `productType` | not in the API | not in the API | Not planned. Workstation versus server | none |
| `reserved` | not in the API | not in the API | Not planned. Reserved by the OS, of no use to an app | none |
| `buildLab` | not in the API | not in the API | Not planned. Registry strings beyond the version numbers | none |
| `buildLabEx` | not in the API | not in the API | Not planned. Same scope line as `buildLab` | none |
| `digitalProductId` | not in the API | not in the API | Not planned. Licensing data | none |
| `displayVersion` | not in the API | not in the API | Not planned. The "21H2" style label; `OsVersion.full` carries the dotted version | none |
| `editionId` | not in the API | not in the API | Not planned. Edition identity | none |
| `installDate` | not in the API | not in the API | Not planned. Install metadata; see `installTime` under `PackageInfo` | none |
| `productId` | not in the API | not in the API | Not planned. Licensing data | none |
| `productName` | not in the API | not in the API | Not planned. The OS edition name; `OsVersion` carries the numbers | none |
| `registeredOwner` | not in the API | not in the API | Not planned. Registrant name; privacy data | none |
| `releaseId` | not in the API | not in the API | Not planned. The "1903" style label; `OsVersion.full` carries the dotted version | none |
| `deviceId` | not in the API | not in the API | Not planned. A machine identifier is tracking data | none |
| not in `WindowsDeviceInfo`: hardware model | `Model.identifier` | `Model.identifier` | Platform only. The reference has no model on Windows; this reads the firmware's `SystemProductName` registry value, raw, and a machine without one reports null | `the_release_keys_name_the_right_fields` (inline in `windows.rs`) |

## `PackageInfo.fromPlatform` and `PackageInfoData`

`PackageInfoData` and `PackageInfo` carry the same fields; one row each. The reference caches the first `fromPlatform` answer in a static; this library re-reads on every call.

| `package_info_plus` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `PackageInfo.fromPlatform()` | `DeviceInfo::app_info()` | `DeviceInfo.appInfo()` | Done. Per call, no singleton. A missing source is a coded error, a missing key is the empty string | `each_call_reads_the_backend_again`; `a_platform_failure_carries_the_backend_code_and_status` |
| `fromPlatform(baseUrl:)` on web | `DeviceInfo::app_info()` (web) | `DeviceInfo.appInfo()` | Partial. Web app info resolves `ERR_DEVICE_INFO_NOT_AVAILABLE`; the app ships its own version constants or fetches its own `version.json` | `every_kind_maps_to_its_stable_code` |
| `fromPlatform()` on Linux | `DeviceInfo::app_info()` (Linux) | `DeviceInfo.appInfo()` | Partial. Desktop Linux app info is the same gap: `ERR_DEVICE_INFO_NOT_AVAILABLE`, no `version.json` reader in v0.1 | `every_kind_maps_to_its_stable_code` |
| `appName` | `AppInfo.app_name` | `AppInfo.appName` | Done. The bundle's display name on Apple (`CFBundleDisplayName`, `CFBundleName` fallback), the application label on Android, the exe's `ProductName` on Windows with the exe name as fallback; the inline tests pin the exact keys | `the_release_keys_name_the_right_fields`; `app_name_falls_back_to_the_exe_stem_when_product_name_is_missing` (inline in `windows.rs`). Canonical #4 is Rust-only inline key-constant tests in `apple.rs`/`android.rs`/`windows.rs`; the TS sentence "app info field names follow the references" has no test — the TS surface has no platform key constants to assert |
| `packageName` | `AppInfo.package_name` | `AppInfo.packageName` | Partial. `bundleIdentifier` on Apple, `applicationId` on Android; Windows has no bundle id, so the exe file name without its extension stands in, where the reference reads `InternalName` out of the version resource | `package_name_is_the_exe_file_name_without_extension` (inline in `windows.rs`) |
| `version` | `AppInfo.version` | `AppInfo.version` | Done. `CFBundleShortVersionString` / `versionName` / the exe's FileVersion | `the_release_keys_name_the_right_fields` |
| `buildNumber` | `AppInfo.build_number` | `AppInfo.buildNumber` | Done. `CFBundleVersion` / `versionCode` / the FileVersion's last dot-separated part; a missing key is the empty string, a missing source is `ERR_DEVICE_INFO_NOT_AVAILABLE` (no loadable bundle on Apple, an unreadable version resource on Windows) | `the_build_number_is_the_last_part_of_the_file_version` (inline in `windows.rs`); `the_release_keys_name_the_right_fields` |
| `buildSignature` | `AppInfo.build_signature` | `AppInfo.buildSignature` | Partial. The Android signing-key signature (hex) where the device gives it; every other target reports null where the reference reports the empty string | `partial_data_is_none_not_an_error` |
| `installerStore` | `AppInfo.installer_store` | `AppInfo.installerStore` | Partial. Android names the installer where the platform does; every other target reports null, as the reference does | `partial_data_is_none_not_an_error` |
| `installTime` | not in the API | not in the API | Not planned. Install metadata, not app version; no NSH API serves it | none |
| `updateTime` | not in the API | not in the API | Not planned. Same scope line as `installTime` | none |
| `PackageInfo.data` | not in the API | not in the API | Not planned. The typed fields are the API | none |
| `setMockInitialValues` (testing) | `MockBackend` + `DeviceInfo::with_backend` | `memoryBackend` (`src/memory.ts`, exported from the index and as `./memory`) | Done. Same seam the reference offers, per query | `the_facade_passes_through_to_its_backend` |

## `DeviceInfoModule` (react-native-device-info): app identity

| `react-native-device-info` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `getApplicationName` | `AppInfo.app_name` | `AppInfo.appName` | Done. Async here, no sync twin | `the_release_keys_name_the_right_fields` |
| `getBuildNumber` | `AppInfo.build_number` | `AppInfo.buildNumber` | Done. Async only | `the_release_keys_name_the_right_fields` |
| `getBundleId` | `AppInfo.package_name` | `AppInfo.packageName` | Done. Async only | `the_release_keys_name_the_right_fields` |
| `getVersion` | `AppInfo.version` | `AppInfo.version` | Done. Async only | `the_release_keys_name_the_right_fields` |
| `getReadableVersion` | not in the API | not in the API | Not planned. The app composes it: `version + "." + buildNumber` from `AppInfo` | none |
| `getInstallerPackageName`, `getInstallerPackageNameSync` | `AppInfo.installer_store` | `AppInfo.installerStore` | Partial. The installer name on Android where the platform gives one; null elsewhere, where the reference says "unknown" | `partial_data_is_none_not_an_error` |
| `getFirstInstallTime`, `getFirstInstallTimeSync` | not in the API | not in the API | Not planned. Install metadata; no NSH API serves it | none |
| `useFirstInstallTime` | not in the API | not in the API | Not planned. NSH has no React; the app reads `AppInfo` once and shows it | none |
| `getLastUpdateTime`, `getLastUpdateTimeSync` | not in the API | not in the API | Not planned. Install metadata | none |
| `getInstallReferrer`, `getInstallReferrerSync` | not in the API | not in the API | Not planned. Install metadata | none |

## `DeviceInfoModule`: model and OS

| `react-native-device-info` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `getModel` | `Model` | `Model` | Partial. The raw identifier plus the platform's own name; no bundled marketing table (the reference maps iOS machine ids through one), and web or Linux give what the platform reports or null where the reference says "unknown" | `model_keeps_the_raw_identifier` |
| `getSystemName` | `DeviceInfo::os()` -> `Os` | `DeviceInfo.os` | Partial. A typed `Os`, compile-time, not the "iOS" / "Android" / "Windows" strings | `the_facade_passes_through_to_its_backend` |
| `getSystemVersion` | `OsVersion` | `OsVersion` | Partial. `major`/`minor`/`full`; Android's `major` is the API level, and web is UA-CH with a best-effort user-agent fallback when the browser withholds it, where the reference returns one string or "unknown" | `the_facade_passes_through_to_its_backend` |
| `getApiLevel`, `getApiLevelSync` | `OsVersion.major` (Android) | `OsVersion.major` | Partial. The API level lands in `major` on Android only | `the_facade_passes_through_to_its_backend` |
| `getDeviceId` | not in the API | not in the API | Not planned. The reference's deviceId is `Build.BOARD` or "unknown", not the hardware model; `Model.identifier` is the model | none |
| `getDeviceName`, `getDeviceNameSync` | not in the API | not in the API | Not planned. The user-assigned device name is privacy data, not model data | none |
| `useDeviceName` | not in the API | not in the API | Not planned. Same scope line as `getDeviceName` | none |
| `getDeviceType` | not in the API | not in the API | Not planned. Handset versus tablet classes; out of scope | none |
| `isTablet` | not in the API | not in the API | Not planned. Same scope line as `getDeviceType` | none |
| `getBrand` | not in the API | not in the API | Not planned. Brand is not model / OS version / app version | none |
| `getManufacturer`, `getManufacturerSync` | not in the API | not in the API | Not planned. Same scope line as `getBrand` | none |
| `useManufacturer` | not in the API | not in the API | Not planned. Same scope line as `getBrand` | none |
| `isEmulator`, `isEmulatorSync` | not in the API | not in the API | Not planned. No boolean: a simulator reports its own model identifier, which the inline test pins | `a_simulator_reports_a_simulator_model` |
| `useIsEmulator` | not in the API | not in the API | Not planned. Same scope line as `isEmulator` | none |
| `hasNotch` | not in the API | not in the API | Not planned. A bundled device-list lookup; out of scope | none |
| `hasDynamicIsland` | not in the API | not in the API | Not planned. Same scope line as `hasNotch` | none |
| `getUserAgent`, `getUserAgentSync` | not in the API | not in the API | Not planned. The app reads `navigator.userAgent` on the web | none |

## `DeviceInfoModule`: the rest

Every remaining member is Not planned — beyond model / OS version / app version — and the row names what the app uses instead.

| `react-native-device-info` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `getAndroidId`, `getAndroidIdSync` | not in the API | not in the API | Not planned. A tracking identifier; the app uses its own account identity | none |
| `getUniqueId`, `getUniqueIdSync` | not in the API | not in the API | Not planned. Same scope line as `getAndroidId` | none |
| `syncUniqueId` | not in the API | not in the API | Not planned. Same scope line as `getAndroidId` | none |
| `getInstanceId`, `getInstanceIdSync` | not in the API | not in the API | Not planned. Same scope line as `getAndroidId` | none |
| `getDeviceToken` | not in the API | not in the API | Not planned. Same scope line as `getAndroidId` | none |
| `getSerialNumber`, `getSerialNumberSync` | not in the API | not in the API | Not planned. Same scope line as `getAndroidId` | none |
| `getAppSetId` | not in the API | not in the API | Not planned. Advertising identity; the app uses its own account identity | none |
| `getBaseOs`, `getBaseOsSync` | not in the API | not in the API | Not planned. Android build strings beyond the version; `OsVersion` carries the API level and `RELEASE` | none |
| `getCodename`, `getCodenameSync` | not in the API | not in the API | Not planned. Same scope line as `getBaseOs` | none |
| `getIncremental`, `getIncrementalSync` | not in the API | not in the API | Not planned. Same scope line as `getBaseOs` | none |
| `getPreviewSdkInt`, `getPreviewSdkIntSync` | not in the API | not in the API | Not planned. Same scope line as `getBaseOs` | none |
| `getSecurityPatch`, `getSecurityPatchSync` | not in the API | not in the API | Not planned. Same scope line as `getBaseOs` | none |
| `getBootloader`, `getBootloaderSync` | not in the API | not in the API | Not planned. `android.os.Build` fields; the host serves them if an app needs one | none |
| `getBuildId`, `getBuildIdSync` | not in the API | not in the API | Not planned. Build and utsname strings; `OsVersion.full` carries the dotted version where one exists | none |
| `getDevice`, `getDeviceSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getDisplay`, `getDisplaySync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getFingerprint`, `getFingerprintSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getHardware`, `getHardwareSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getHost`, `getHostSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getHostNames`, `getHostNamesSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getProduct`, `getProductSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getTags`, `getTagsSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getType`, `getTypeSync` | not in the API | not in the API | Not planned. Same scope line as `getBootloader` | none |
| `getBatteryLevel`, `getBatteryLevelSync` | not in the API | not in the API | Not planned. Power state; web apps use `navigator.getBattery()`, elsewhere no NSH API | none |
| `useBatteryLevel` | not in the API | not in the API | Not planned. NSH has no React; the app polls whatever power API it has | none |
| `useBatteryLevelIsLow` | not in the API | not in the API | Not planned. Same scope line as `useBatteryLevel` | none |
| `isBatteryCharging`, `isBatteryChargingSync` | not in the API | not in the API | Not planned. Same scope line as `getBatteryLevel` | none |
| `getPowerState`, `getPowerStateSync` | not in the API | not in the API | Not planned. Power state | none |
| `usePowerState` | not in the API | not in the API | Not planned. Power state | none |
| `getBrightness`, `getBrightnessSync` | not in the API | not in the API | Not planned. Display state; no NSH API | none |
| `useBrightness` | not in the API | not in the API | Not planned. Display state | none |
| `getFreeDiskStorage`, `getFreeDiskStorageSync` | not in the API | not in the API | Not planned. Storage; the platform's own file-system APIs | none |
| `getFreeDiskStorageOld`, `getFreeDiskStorageOldSync` | not in the API | not in the API | Not planned. Same scope line as `getFreeDiskStorage` | none |
| `getTotalDiskCapacity`, `getTotalDiskCapacitySync` | not in the API | not in the API | Not planned. Storage | none |
| `getTotalDiskCapacityOld`, `getTotalDiskCapacityOldSync` | not in the API | not in the API | Not planned. Same scope line as `getTotalDiskCapacity` | none |
| `getMaxMemory`, `getMaxMemorySync` | not in the API | not in the API | Not planned. Memory; the platform's own process APIs | none |
| `getTotalMemory`, `getTotalMemorySync` | not in the API | not in the API | Not planned. Memory | none |
| `getUsedMemory`, `getUsedMemorySync` | not in the API | not in the API | Not planned. Memory | none |
| `getCarrier`, `getCarrierSync` | not in the API | not in the API | Not planned. Telephony; no NSH API | none |
| `getIpAddress`, `getIpAddressSync` | not in the API | not in the API | Not planned. Network interfaces; the platform's own socket APIs | none |
| `getMacAddress`, `getMacAddressSync` | not in the API | not in the API | Not planned. Same scope line as `getIpAddress` | none |
| `getAvailableLocationProviders`, `getAvailableLocationProvidersSync` | not in the API | not in the API | Not planned. Location is a location capability, not device identity | none |
| `isLocationEnabled`, `isLocationEnabledSync` | not in the API | not in the API | Not planned. Same scope line as `getAvailableLocationProviders` | none |
| `getFontScale`, `getFontScaleSync` | not in the API | not in the API | Not planned. Typography; the shell's own text settings | none |
| `supported32BitAbis`, `supported32BitAbisSync` | not in the API | not in the API | Not planned. ABI inventory; the build target knows the ABI at compile time | none |
| `supported64BitAbis`, `supported64BitAbisSync` | not in the API | not in the API | Not planned. Same scope line as `supported32BitAbis` | none |
| `supportedAbis`, `supportedAbisSync` | not in the API | not in the API | Not planned. Same scope line as `supported32BitAbis` | none |
| `getSystemAvailableFeatures`, `getSystemAvailableFeaturesSync` | not in the API | not in the API | Not planned. The Android package manager serves feature questions | none |
| `hasSystemFeature`, `hasSystemFeatureSync` | not in the API | not in the API | Not planned. Same scope line as `getSystemAvailableFeatures` | none |
| `useHasSystemFeature` | not in the API | not in the API | Not planned. Same scope line as `getSystemAvailableFeatures` | none |
| `getSupportedMediaTypeList`, `getSupportedMediaTypeListSync` | not in the API | not in the API | Not planned. Media capabilities; the platform's codec APIs | none |
| `hasGms`, `hasGmsSync` | not in the API | not in the API | Not planned. Google service detection; no NSH API | none |
| `hasHms`, `hasHmsSync` | not in the API | not in the API | Not planned. Huawei service detection; no NSH API | none |
| `isAirplaneMode`, `isAirplaneModeSync` | not in the API | not in the API | Not planned. Device state; no NSH API | none |
| `isPinOrFingerprintSet`, `isPinOrFingerprintSetSync` | not in the API | not in the API | Not planned. The NSH biometrics capability answers credential questions | none |
| `isCameraPresent`, `isCameraPresentSync` | not in the API | not in the API | Not planned. Device state; no NSH API | none |
| `isHeadphonesConnected`, `isHeadphonesConnectedSync` | not in the API | not in the API | Not planned. Audio hardware; no NSH API | none |
| `isWiredHeadphonesConnected`, `isWiredHeadphonesConnectedSync` | not in the API | not in the API | Not planned. Same scope line as `isHeadphonesConnected` | none |
| `isBluetoothHeadphonesConnected`, `isBluetoothHeadphonesConnectedSync` | not in the API | not in the API | Not planned. Same scope line as `isHeadphonesConnected` | none |
| `useIsHeadphonesConnected` | not in the API | not in the API | Not planned. Same scope line as `isHeadphonesConnected` | none |
| `useIsWiredHeadphonesConnected` | not in the API | not in the API | Not planned. Same scope line as `isHeadphonesConnected` | none |
| `useIsBluetoothHeadphonesConnected` | not in the API | not in the API | Not planned. Same scope line as `isHeadphonesConnected` | none |
| `isLandscape`, `isLandscapeSync` | not in the API | not in the API | Not planned. Orientation is the shell's own view state | none |
| `isMouseConnected`, `isMouseConnectedSync` | not in the API | not in the API | Not planned. Input hardware; the shell's own input events | none |
| `isKeyboardConnected`, `isKeyboardConnectedSync` | not in the API | not in the API | Not planned. Same scope line as `isMouseConnected` | none |
| `isTabletMode` | not in the API | not in the API | Not planned. Same scope line as `isMouseConnected` | none |
| `isLowRamDevice` | not in the API | not in the API | Not planned. Device class flags; out of scope | none |
| `isDisplayZoomed` | not in the API | not in the API | Not planned. Same scope line as `isLowRamDevice` | none |
| `getStartupTime`, `getStartupTimeSync` | not in the API | not in the API | Not planned. Process start; the platform's own clock | none |

## Failures

The references throw `PlatformException` or reject with errors. This library resolves every call exactly once with a value or a coded error, and the codes are stable strings on both sides of the bridge.

| Reference behaviour | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| A platform source fails (`PlatformException`, a rejected promise) | `DeviceInfoError`, code `ERR_DEVICE_INFO_BACKEND`, platform status kept when there is one | rejects with the code `ERR_DEVICE_INFO_BACKEND` | Done | `a_platform_failure_carries_the_backend_code_and_status`; `every_kind_maps_to_its_stable_code` |
| The reference's "unknown" / null defaults on a platform with no source | `Err` with code `ERR_DEVICE_INFO_NOT_AVAILABLE`: the unsupported target, web app info, Linux app info | rejects with the code `ERR_DEVICE_INFO_NOT_AVAILABLE` | Done. Missing data inside an answer is null, not an error | `an_unsupported_target_reports_not_available`; `partial_data_is_none_not_an_error`; `every_kind_maps_to_its_stable_code` |

Added here, not in the references: the `Backend` trait, the Rust `MockBackend` and the TypeScript `memoryBackend` (tests inject answers and count reads), and the no-cache rule itself.
