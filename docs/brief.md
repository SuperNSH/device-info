# Device info: brief

Device model, OS version and app version for Native Shell apps.

## Options

- os_info: Rust, OS type, version and architecture
- device-info crate: Rust hardware model and device name (verify the OS version field before relying on it)
- Native: UIDevice.model on iOS; Build.MODEL on Android
- Reference only: device_info_plus, react-native-device-info

## Recommended path

os_info and the device-info crate on desktop. Native model name on iOS and Android. App version from the build metadata.

## Edge cases to design for

- Model identifiers are not marketing names; map them or show the raw value
- Simulators report a simulator model
- App version must match CFBundleShortVersionString and versionName
- OS version values can change between releases; read them at each call

## Research status

Research done 2026-11-09 by web search. Versions and licences are unverified until checked on the registry before pinning. Full catalogue: the research notes for the SuperNSH library set.
