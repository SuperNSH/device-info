//! The WebAssembly bridge. `model` and `platformVersion` are high-entropy UA-CH hints, so with
//! UA-CH the browser answers through `navigator.userAgentData.getHighEntropyValues` — one
//! awaited promise per call, and a rejected promise is `ERR_DEVICE_INFO_BACKEND`. Without UA-CH
//! the OS version is parsed out of
//! `navigator.userAgent`, best-effort: browsers freeze or reduce that string, so it is a hint,
//! not a guarantee. `app_info` resolves `ERR_DEVICE_INFO_NOT_AVAILABLE` because under NSH 0.4.0
//! no build metadata (bundle, version, build number) is reachable from wasm — the same call the
//! biometrics reference made for the web. The parse helpers sit behind
//! `cfg(any(target_family = "wasm", test))` — they serve the wasm reads and the unit tests below,
//! which therefore run on the host without a browser; only `mod browser` touches JS.

#[cfg(any(target_family = "wasm", test))]
use crate::info::{Model, OsVersion};

#[cfg(any(target_family = "wasm", test))]
/// User-agent segments that name an OS. iPadOS reports "CPU OS" and iOS segments also say
/// "like Mac OS X", so the first matching segment in the string wins.
const OS_MARKERS: [&str; 6] = [
    "iPhone OS",
    "CPU OS",
    "Windows Phone",
    "Windows NT",
    "Android",
    "Mac OS X",
];

#[cfg(any(target_family = "wasm", test))]
/// The user-agent segment naming the OS, e.g. "Android 13" or "Intel Mac OS X 10_15_7".
fn os_token(user_agent: &str) -> Option<&str> {
    user_agent
        .split([';', '(', ')'])
        .map(str::trim)
        .find(|segment| OS_MARKERS.iter().any(|marker| segment.contains(marker)))
}

#[cfg(any(target_family = "wasm", test))]
/// The first two whole numbers in `text` — 0 when missing, saturating at `u16::MAX`. Version
/// parts arrive separated by '.', '_' or ' ', so every maximal digit run is one part.
fn first_two_numbers(text: &str) -> (u16, u16) {
    let cap = u64::from(u16::MAX);
    let mut slots = [0u16; 2];
    let mut slot = 0;
    let mut run: u64 = 0;
    let mut in_run = false;
    for character in text.chars() {
        match character.to_digit(10) {
            Some(digit) => {
                run = run.saturating_mul(10).saturating_add(u64::from(digit)).min(cap);
                in_run = true;
            }
            None if in_run => {
                slots[slot] = run as u16;
                slot += 1;
                run = 0;
                in_run = false;
                if slot == slots.len() {
                    break;
                }
            }
            None => {}
        }
    }
    if in_run && slot < slots.len() {
        slots[slot] = run as u16;
    }
    (slots[0], slots[1])
}

#[cfg(any(target_family = "wasm", test))]
/// A UA-CH `platformVersion` ("15.0.0") into major/minor, keeping the string as `full`; absent
/// or reduced to empty is `(0, 0, None)` — partial data, not an error.
fn os_version_from_platform_version(version: Option<&str>) -> OsVersion {
    match version.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => {
            let (major, minor) = first_two_numbers(value);
            OsVersion { major, minor, full: Some(value.to_owned()) }
        }
        None => OsVersion { major: 0, minor: 0, full: None },
    }
}

#[cfg(any(target_family = "wasm", test))]
/// The best-effort OS version out of a user agent; no OS token at all is `(0, 0, None)`.
fn os_version_from_user_agent(user_agent: &str) -> OsVersion {
    match os_token(user_agent) {
        Some(token) => {
            let (major, minor) = first_two_numbers(token);
            OsVersion { major, minor, full: Some(token.to_owned()) }
        }
        None => OsVersion { major: 0, minor: 0, full: None },
    }
}

#[cfg(any(target_family = "wasm", test))]
/// A UA-CH `model` into a `Model`: the raw identifier only, since UA-CH carries no marketing
/// name. Absent or reduced to empty is `Model { None, None }` — partial data, not an error.
fn model_from_user_agent_data(model: Option<&str>) -> Model {
    Model {
        identifier: model
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        name: None,
    }
}

#[cfg(target_family = "wasm")]
mod browser {
    // web-sys ships NavigatorUaData only behind --cfg=web_sys_unstable_apis, which no NSH build
    // passes, so the same objects are reached with js-sys reflection over the navigator.
    use super::{
        model_from_user_agent_data, os_version_from_platform_version, os_version_from_user_agent,
    };
    use crate::backend::{Backend, BackendFuture};
    use crate::error::{DeviceInfoError, ErrorKind};
    use crate::info::{AppInfo, Model, OsVersion};
    use js_sys::{Array, Function, Promise, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    pub(crate) struct NativeBackend;

    /// `navigator.userAgentData`, absent on Safari and Firefox and outside a DOM.
    fn user_agent_data() -> Option<JsValue> {
        let navigator: JsValue = web_sys::window()?.navigator().into();
        let data = Reflect::get(&navigator, &JsValue::from_str("userAgentData")).ok()?;
        if data.is_undefined() || data.is_null() {
            None
        } else {
            Some(data)
        }
    }

    /// `object.name` as a JS string; a missing or non-string value is absent data, not a failure.
    fn string_property(object: &JsValue, name: &str) -> Option<String> {
        Reflect::get(object, &JsValue::from_str(name)).ok()?.as_string()
    }

    /// `navigator.userAgent`, the OS fallback when UA-CH is missing.
    fn user_agent() -> Option<String> {
        let navigator: JsValue = web_sys::window()?.navigator().into();
        string_property(&navigator, "userAgent")
    }

    /// Awaits `getHighEntropyValues(fields)` once. `Ok(None)` — no DOM, no UA-CH, no method;
    /// `Err` — the call or the promise failed, a backend failure.
    async fn high_entropy(fields: &[&str]) -> Result<Option<JsValue>, ()> {
        let data = match user_agent_data() {
            Some(data) => data,
            None => return Ok(None),
        };
        let function = match Reflect::get(&data, &JsValue::from_str("getHighEntropyValues")) {
            Ok(value) => value.dyn_into::<Function>().ok(),
            Err(_) => None,
        };
        let function = match function {
            Some(function) => function,
            None => return Ok(None),
        };
        let hints = Array::new();
        for field in fields {
            hints.push(&JsValue::from_str(field));
        }
        let promise = function.call1(&data, &hints).map_err(|_| ())?;
        let promise = promise.dyn_into::<Promise>().map_err(|_| ())?;
        let values = JsFuture::from(promise).await.map_err(|_| ())?;
        Ok(Some(values))
    }

    impl Backend for NativeBackend {
        fn device_model(&self) -> BackendFuture<'_, Result<Model, DeviceInfoError>> {
            Box::pin(async {
                match high_entropy(&["model"]).await {
                    Ok(values) => {
                        let model = values.as_ref().and_then(|v| string_property(v, "model"));
                        Ok(model_from_user_agent_data(model.as_deref()))
                    }
                    Err(()) => Err(DeviceInfoError::new(ErrorKind::Backend)),
                }
            })
        }

        fn os_version(&self) -> BackendFuture<'_, Result<OsVersion, DeviceInfoError>> {
            Box::pin(async {
                match high_entropy(&["platformVersion"]).await {
                    Ok(Some(values)) => {
                        let version = string_property(&values, "platformVersion");
                        Ok(os_version_from_platform_version(version.as_deref()))
                    }
                    Ok(None) => {
                        let agent = user_agent().unwrap_or_default();
                        Ok(os_version_from_user_agent(&agent))
                    }
                    Err(()) => Err(DeviceInfoError::new(ErrorKind::Backend)),
                }
            })
        }

        fn app_info(&self) -> BackendFuture<'_, Result<AppInfo, DeviceInfoError>> {
            Box::pin(async { Err(DeviceInfoError::new(ErrorKind::NotAvailable)) })
        }
    }
}

#[cfg(target_family = "wasm")]
pub(crate) use browser::NativeBackend;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_version_parses_major_and_minor() {
        let parsed = os_version_from_platform_version(Some("15.0.0"));
        assert_eq!(parsed, OsVersion { major: 15, minor: 0, full: Some("15.0.0".to_owned()) });
        let parsed = os_version_from_platform_version(Some("10.15.7"));
        assert_eq!(parsed, OsVersion { major: 10, minor: 15, full: Some("10.15.7".to_owned()) });
    }

    #[test]
    fn platform_version_reduced_or_absent_is_zeros() {
        let parsed = os_version_from_platform_version(None);
        assert_eq!(parsed, OsVersion { major: 0, minor: 0, full: None });
        let parsed = os_version_from_platform_version(Some(""));
        assert_eq!(parsed, OsVersion { major: 0, minor: 0, full: None });
        let parsed = os_version_from_platform_version(Some("   "));
        assert_eq!(parsed, OsVersion { major: 0, minor: 0, full: None });
    }

    #[test]
    fn platform_version_saturates_past_u16() {
        let parsed = os_version_from_platform_version(Some("70000.2.0"));
        assert_eq!(parsed.major, u16::MAX);
        assert_eq!(parsed.minor, 2);
        assert_eq!(parsed.full.as_deref(), Some("70000.2.0"));
    }

    #[test]
    fn platform_version_keeps_an_unparseable_string() {
        let parsed = os_version_from_platform_version(Some("Unknown"));
        assert_eq!(parsed, OsVersion { major: 0, minor: 0, full: Some("Unknown".to_owned()) });
    }

    #[test]
    fn user_agent_tokens_parse_per_platform() {
        let agent = "Mozilla/5.0 (Linux; Android 13; Pixel 7) Chrome/131.0 Mobile Safari/537.36";
        let parsed = os_version_from_user_agent(agent);
        assert_eq!(parsed.major, 13);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.full.as_deref(), Some("Android 13"));

        let agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5_1 like Mac OS X) Safari/604.1";
        let parsed = os_version_from_user_agent(agent);
        assert_eq!(parsed.major, 17);
        assert_eq!(parsed.minor, 5);
        assert_eq!(parsed.full.as_deref(), Some("CPU iPhone OS 17_5_1 like Mac OS X"));

        let agent = "Mozilla/5.0 (iPad; CPU OS 16_6 like Mac OS X) Safari/604.1";
        let parsed = os_version_from_user_agent(agent);
        assert_eq!(parsed.major, 16);
        assert_eq!(parsed.minor, 6);
        assert_eq!(parsed.full.as_deref(), Some("CPU OS 16_6 like Mac OS X"));

        let agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15";
        let parsed = os_version_from_user_agent(agent);
        assert_eq!(parsed.major, 10);
        assert_eq!(parsed.minor, 15);
        assert_eq!(parsed.full.as_deref(), Some("Intel Mac OS X 10_15_7"));

        let agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/131.0 Safari/537.36";
        let parsed = os_version_from_user_agent(agent);
        assert_eq!(parsed.major, 10);
        assert_eq!(parsed.minor, 0);
        assert_eq!(parsed.full.as_deref(), Some("Windows NT 10.0"));
    }

    #[test]
    fn user_agent_without_an_os_token_reports_nothing() {
        // Linux agents carry no kernel version, so they are "nothing at all".
        let agent = "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0";
        let parsed = os_version_from_user_agent(agent);
        assert_eq!(parsed, OsVersion { major: 0, minor: 0, full: None });
        let parsed = os_version_from_user_agent("curl/8.4.0");
        assert_eq!(parsed, OsVersion { major: 0, minor: 0, full: None });
    }

    #[test]
    fn model_extraction_keeps_the_raw_string() {
        let model = model_from_user_agent_data(Some("SM-S928B"));
        assert_eq!(model, Model { identifier: Some("SM-S928B".to_owned()), name: None });
        let model = model_from_user_agent_data(None);
        assert_eq!(model, Model { identifier: None, name: None });
        let model = model_from_user_agent_data(Some("  "));
        assert_eq!(model, Model { identifier: None, name: None });
    }
}
