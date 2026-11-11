import type { AppInfo, Model, OsVersion } from "./types";

/** What a `DeviceInfo` runs on: the Rust crate, or a memory backend in tests. */
export interface DeviceInfoBackend {
  deviceModel(): Promise<Model>;
  osVersion(): Promise<OsVersion>;
  appInfo(): Promise<AppInfo>;
}
