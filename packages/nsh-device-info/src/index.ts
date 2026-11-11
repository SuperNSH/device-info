import { platform } from "nsh";
import type { DeviceInfoBackend } from "./backend";
import { rustBackend } from "./rust";
import type { AppInfo, Model, Os, OsVersion } from "./types";

/**
 * The device's model, OS version and app version. It runs on the Rust crate unless it is given
 * another backend. Nothing is cached: an OS version can change between calls, so each call reads
 * the backend again. Every rejection carries a `DeviceInfoErrorCode`.
 */
export class DeviceInfo {
  private readonly backend: DeviceInfoBackend;

  constructor(backend: DeviceInfoBackend = rustBackend()) {
    this.backend = backend;
  }

  /** The OS the app runs on, from the runtime's `platform.os`. Constant while the app runs. */
  os(): Os {
    return platform.os;
  }

  deviceModel(): Promise<Model> {
    return this.backend.deviceModel();
  }

  osVersion(): Promise<OsVersion> {
    return this.backend.osVersion();
  }

  appInfo(): Promise<AppInfo> {
    return this.backend.appInfo();
  }
}

export { type DeviceInfoBackend } from "./backend";
export { DeviceInfoError, asDeviceInfoError, deviceInfoError, type DeviceInfoErrorCode } from "./errors";
export { memoryBackend, type MemoryBackend, type MemoryDeviceInfoConfig } from "./memory";
export { rustBackend } from "./rust";
export { type AppInfo, type Model, type Os, type OsVersion } from "./types";
