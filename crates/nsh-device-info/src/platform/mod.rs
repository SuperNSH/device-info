//! The platform backends. Each target gets one `NativeBackend`; a target with none reports that
//! nothing is available.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_vendor = "apple")]
mod apple;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(any(
    target_vendor = "apple",
    target_os = "android",
    target_os = "linux",
    target_os = "windows",
    target_family = "wasm"
)))]
mod unsupported;
// web's pure helpers also compile under `test` so their unit tests run on the host; its
// NativeBackend stays wasm-only, so every non-wasm build still answers through unsupported.
#[cfg(any(target_family = "wasm", test))]
mod web;
#[cfg(target_os = "windows")]
mod windows;

use std::sync::Arc;

use crate::backend::Backend;

#[cfg(target_os = "android")]
use android::NativeBackend;
#[cfg(target_vendor = "apple")]
use apple::NativeBackend;
#[cfg(target_os = "linux")]
use linux::NativeBackend;
#[cfg(not(any(
    target_vendor = "apple",
    target_os = "android",
    target_os = "linux",
    target_os = "windows",
    target_family = "wasm"
)))]
use unsupported::NativeBackend;
#[cfg(target_family = "wasm")]
use web::NativeBackend;
#[cfg(target_os = "windows")]
use windows::NativeBackend;

/// The backend of the target this build runs on.
pub(crate) fn default_backend() -> Arc<dyn Backend> {
    Arc::new(NativeBackend)
}
