import { appInfo as buildInfo, platform } from "nsh";
import { NativeDeviceInfo } from "nsh-device-info/native";
import type { DeviceInfoBackend } from "./backend";
import { appInfoWithBuildFallback } from "./build";
import { asDeviceInfoError } from "./errors";
import type { AppInfo, Model, OsVersion } from "./types";

/**
 * The Rust crate's device information (`nsh-device-info/native`). The app or package must list the
 * crate under `"rust"` in `nsh.json`; importing this module without it fails the bundle. On web and
 * desktop Linux, where the crate has no app info source, `appInfo()` answers from the runtime's build metadata.
 */
export function rustBackend(): DeviceInfoBackend {
  return new RustBackend();
}

class RustBackend implements DeviceInfoBackend {
  private readonly native = new NativeDeviceInfo();

  // The crate's generated types mark `Option` fields optional; at run time it always sends them (null for None).
  deviceModel(): Promise<Model> {
    return this.settle(async () => (await this.native.deviceModel()) as Model);
  }

  osVersion(): Promise<OsVersion> {
    return this.settle(async () => (await this.native.osVersion()) as OsVersion);
  }

  appInfo(): Promise<AppInfo> {
    return this.settle(() =>
      appInfoWithBuildFallback(async () => (await this.native.appInfo()) as AppInfo, platform.os, buildInfo),
    );
  }

  private async settle<T>(operation: () => Promise<T>): Promise<T> {
    try {
      return await operation();
    } catch (error) {
      throw asDeviceInfoError(error);
    }
  }
}
