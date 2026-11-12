# Decisions

Each entry says what was chosen and why. [goals.md](goals.md) states the acceptance criteria; [parity.md](parity.md) lists what each reference API does on each platform. The three items this file opened with — the NSH version, the library choice, the platforms in scope — are decided below.

## NSH 0.5.0 is the pinned version

This repo's brief names no NSH version; the pin comes from the SuperNSH set's brief ("NSH crates are `0.4.0`", `.refs/biometrics/docs/brief.md`), whose research notes this repo's brief defers to. The library stays on the released crate: it started on `nsh-core = "0.4.0"`, the version SuperNSH/biometrics pinned, and moved to `nsh-core = "0.5.0"` from crates.io, the version CI installs, with the SuperNSH set on 2026-11-11. Nothing here needs the unreleased `main` branch: the reads need no manifest key and no permission — `utsname`, `UIDevice` and `NSBundle` on Apple, `android.os.Build` and the package manager on Android, an `HKLM` registry read on Windows, the DMI files under `/sys` on Linux, UA-CH in the browser. The `ios.infoPlist` trouble biometrics hit does not apply.

## os_info 3 for the desktop version, native APIs for the model

The brief's recommended path was os_info and the device-info crate on desktop. The split:

- OS version on macOS, Windows and Linux: os_info 3. Its `Version` arrives parsed — `Semantic(m, n, p)`, `Rolling`, `Custom`, `Unknown` — which is the shape `OsVersion` needs; the three desktop bridges convert through one shared `from_os_info` in `info.rs`, with inline tests.
- Model on every platform: the platform's own API — `utsname` and `sysctl` on Apple, `android.os.Build` on Android, the Win32 registry and VERSIONINFO on Windows, the DMI product name on Linux, UA-CH `model` on the web.
- The device-info crate is rejected. The brief says its OS version field is unverified, and nothing verified it. An unverified version field is not worth one syscall avoided: os_info answers the version, and each model read is a single native call.

## Six platforms in scope, and what each reports

The first release covers all six NSH surfaces; "desktop" is three of them.

| Platform | Model | OS version | App info |
|---|---|---|---|
| iOS | `utsname.machine`, raw — a simulator reports its own machine string, not a fake iPhone; no name in v0.1, since `UIDevice.model` only gives the class string "iPhone" | `UIDevice.systemVersion`: major, minor, the patch appended to the full string only when nonzero | NSBundle: display name, bundle id, `CFBundleShortVersionString`, `CFBundleVersion` |
| Android | `Build.MODEL`, raw | `Build.VERSION.SDK_INT` in `major`, 0 in `minor`, `RELEASE` as the full string | the package manager: label, package name, `versionName`, `versionCode`, installer store, signing signature |
| macOS | sysctl `hw.model`, raw; no name — the platform provides none | os_info | NSBundle, as on iOS |
| Windows | the registry's machine product name, raw | os_info | the exe's FileVersion |
| Linux | the DMI product name; a machine without one (a container) reports `None` | os_info, `Custom` and `Rolling` strings kept | the gap below |
| web | UA-CH `model` where the browser serves it, else `None` | UA-CH `platformVersion`, parsed; when the browser withholds UA-CH, a best-effort `navigator.userAgent` parse; reduced or absent gives (0, 0, the string or `None`) | the gap below |

Any other target compiles the unsupported backend: every query resolves `ERR_DEVICE_INFO_NOT_AVAILABLE`.

## The layout is the one biometrics uses

The workspace root `Cargo.toml` lists `members = ["crates/nsh-device-info"]`; the crate is `crates/nsh-device-info` with `[lib] name = "nsh_device_info"`; the TypeScript package is `packages/nsh-device-info`. Copied from SuperNSH/biometrics, so the SuperNSH set reads as one.

## Raw identifiers; no marketing-name table in v0.1

`Model.identifier` is the hardware string exactly as the platform reports it. `Model.name` carries a marketing name only when the platform itself provides one, and none does in scope: `UIDevice.model` only gives the class string "iPhone", and macOS, Android, Windows, Linux and the web give no name of their own — so `name` is `None` on every platform in v0.1. The references bundle tables (device_info_plus maps through `DeviceIdentifiers` and `getMacModelName`; react-native-device-info maps iOS machine ids), and react-native-device-info's README warns the community list "could lag new devices" and recommends the raw identifier. A bundled table inherits that staleness the day it is merged, so v0.1 ships none. Missing data is `None`, never an error.

## Reads are never cached

device_info_plus documents each cached answer as "This information does not change from call to call. Cache it." and package_info_plus keeps the first `fromPlatform` answer in a static. An OS update changes the version while the app runs, so the premise fails here: every call reads the backend again. `MockBackend` counts reads, so the tests see the re-read.

## Two error codes, and no more

- `ERR_DEVICE_INFO_NOT_AVAILABLE` — the target has no source for the answer.
- `ERR_DEVICE_INFO_BACKEND` — the source exists and the read failed.

That is every failure the library can meet. There are no arguments to refuse (no invalid-argument code), no system prompt to dismiss (no cancel code) and nothing slow enough to time out — each call is one platform read. The same two codes and the same English messages appear on the TypeScript side; the platform status rides along when the platform reports one.

## App info on web and desktop Linux: named gaps

- Web: the browser has no bundle. package_info_plus answers from a `version.json` its build generates and the app fetches; NSH 0.4.0 generates no such artifact (0.5.0's TypeScript `appInfo()` reads build metadata on the web; the Rust crate does not use it yet). `app_info` resolves `ERR_DEVICE_INFO_NOT_AVAILABLE`; an app ships its own constants today. A later version needs the NSH build to emit a version artifact, and a reader for it.
- Desktop Linux: no bundle, and no package manager owns an exe in `/usr/local`. A later version needs a build-time source — NSH injecting the app's metadata at packaging, or parsing the `.desktop` file the installer ships.

Model and OS version work on both; only app info is the gap. Both rows are Partial in [parity.md](parity.md).

## Versions

Each direct dependency is the newest stable release confirmed on crates.io at implementation time; `crates/nsh-device-info/Cargo.toml` is the record. The lines the code needs: nsh-core 0.5.0, thiserror 2, os_info 3 on the desktop targets, objc2-foundation 0.3 and libc on Apple, objc2-ui-kit 0.3 on iOS, jni 0.22 and ndk-context 0.1 on Android, windows 0.62 on Windows, web-sys 0.3 / js-sys 0.3 / wasm-bindgen 0.2 / wasm-bindgen-futures 0.4 on wasm — 0.4, not 0.2, because the futures crate's 0.2 series is unbuildable here (0.2.22 predates `JsFuture` and hard-depends on js-sys 0.2 / futures 0.1), and 0.4 is the lockstep series of the wasm-bindgen 0.2 / js-sys 0.3 pins. web-sys itself ships only `Navigator` and `Window`: its UA-CH types are gated behind `--cfg=web_sys_unstable_apis`, which no NSH build passes, so `platform/web.rs` reaches `navigator.userAgentData` through js-sys `Reflect`.

## References

Cloned read-only into `.refs/` on 2026-11-09. All five succeeded; no substitute was needed.

| Repo | Canonical URL | Cloned at |
|---|---|---|
| SuperNSH/biometrics | https://github.com/SuperNSH/biometrics | `fe622d8` |
| nsh | https://github.com/longbridge/nsh | `5fbac32` (0.4.0) |
| os_info | https://github.com/stanislav-tkach/os_info | `485b848` (3.15.0) |
| plus_plugins | https://github.com/fluttercommunity/plus_plugins | `907994b` (device_info_plus 13.3.0, package_info_plus 10.2.2) |
| react-native-device-info | https://github.com/react-native-device-info/react-native-device-info | `06a9ed6` (15.0.2) |
