# Device info: goals

This library answers three questions for a Native Shell app: which device (`Model`), which OS (`OsVersion`), which app (`AppInfo`). What was chosen and why is in [decisions.md](decisions.md); what each reference API does here is in [parity.md](parity.md).

## Goals

The library reports device model, OS version and app version for Native Shell apps, on iOS, Android, macOS, Windows, Linux and web. Every behavior exists twice — in the Rust crate at `crates/nsh-device-info` and the TypeScript package at `packages/nsh-device-info` — with a test in both. The TypeScript side is the `nsh-device-info/native` module over the same Rust facade, not a second implementation: a behavior counts as done when both tests pin it.

## Acceptance criteria

- Every call resolves exactly once: a value or a coded error, never both, never neither. These are quick platform reads with nothing for the user to dismiss, so there is no cancel path — and none is owed.
- No panic and no hang on any input. The library takes no arguments; the inputs that can hurt it are the platform's own answers — a missing bundle key, an unreadable source, a reduced browser. Each lands in a value, a `None`/`null`, or a coded error.
- No caching. An OS version can change between releases while the app runs, so every call reads the backend again; the facade keeps no memo and no field.
- Each behavior has its named test in Rust (`crates/nsh-device-info/tests/`, plus the inline tests the table below names) and in TypeScript (`packages/nsh-device-info/tests/`).
- The gates pass on GitHub CI and never run on a developer machine: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `nsh check`, `nsh test`. A claim with no passing run is unverified.

## Errors

`ErrorKind` has two kinds, and each maps to a stable code and one English message — the same strings on both sides of the bridge.

| Kind | Code | Message | When |
|---|---|---|---|
| `NotAvailable` | `ERR_DEVICE_INFO_NOT_AVAILABLE` | Device information is not available on this platform. | The target has no source for the answer: any target beyond the six, app info on web and desktop Linux. |
| `Backend` | `ERR_DEVICE_INFO_BACKEND` | The platform's device information service failed. | The source exists and the read failed. |

`DeviceInfoError` carries the kind, the code and an optional platform status (`status()`; `with_status(i64)` sets it), kept when the platform reports one. Display shows the message and nothing else: no reason text, no user data. A TypeScript call rejects with the same code and message.

## Edge cases

One row per canonical test; the test names are the map the other docs cite. Rust names are the snake_case functions, in `crates/nsh-device-info/tests/` unless the row names an inline test; TypeScript names are the `it("...")` sentences in `packages/nsh-device-info/tests/`.

| # | Edge case | Rust test | TypeScript test |
|---|---|---|---|
| 1 | OS versions change between releases; read at each call. The references cache — device_info_plus documents each answer "This information does not change from call to call. Cache it.", package_info_plus keeps the first `fromPlatform` in a static — this library re-reads, and the mock backend's counters show two reads. | `each_call_reads_the_backend_again` | "each call reads the backend again" |
| 2 | Model identifiers are not marketing names. The references bundle mapping tables and react-native-device-info's README warns the list "could lag new devices"; v0.1 reports the raw string ("iPhone16,2", "SM-S928B") and bundles no table. | `model_keeps_the_raw_identifier` | "model keeps the raw identifier" |
| 3 | Simulators report a simulator model — device_info_plus substitutes `SIMULATOR_MODEL_IDENTIFIER`, react-native-device-info's emulator reports "goldfish". Here the platform's answer stands, and there is no `isEmulator` boolean. | `a_simulator_reports_a_simulator_model` (inline in `apple.rs`) | platform only |
| 4 | App version must match `CFBundleShortVersionString` and `versionName`. The keys are exactly what package_info_plus reads; a missing key is the empty string, a missing source is a coded error. | `the_release_keys_name_the_right_fields` (inline key-constant tests in `apple.rs`, `android.rs`, `windows.rs`) | no TS test — Rust-only inline key-constant tests |
| 5 | A target beyond the six has no source: the unsupported backend answers every query with `NotAvailable`. | `an_unsupported_target_reports_not_available` | "an unsupported target reports not available" |
| 6 | A platform read fails. The references throw a `PlatformException` or reject; here the call resolves `ERR_DEVICE_INFO_BACKEND` with the platform status kept when there is one. | `a_platform_failure_carries_the_backend_code_and_status` | "a platform failure carries the backend code and status" |
| 7 | Partial data: a reduced browser, a container without DMI, a Mac with no name of its own. Missing data is `None`/`null`, never an error. | `partial_data_is_none_not_an_error` | "partial data is null not an error" |
| 8 | Every kind maps to its stable code, identically in Rust and TypeScript. | `every_kind_maps_to_its_stable_code` | "every kind maps to its stable code" |
| 9 | The facade passes through to its backend: `DeviceInfo::new()` picks the platform default, `with_backend` swaps it, and every answer is the backend's. | `the_facade_passes_through_to_its_backend` | "the facade passes through to its backend" |
| 10 | Unknown and rolling versions keep their string: os_info's `Custom` and `Rolling`, a UA-CH `platformVersion` that parses to nothing — `(0, 0, the string or None)`. | `unknown_versions_keep_their_string` (in `tests/facade.rs`) | "unknown versions keep their string" |

## Non-goals

Battery, RAM, disk, device name, unique identifiers, permissions, locale, first-install time: not this library. [parity.md](parity.md) marks each such reference API as Not planned and names what the app uses instead.
