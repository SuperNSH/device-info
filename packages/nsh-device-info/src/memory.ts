import type { DeviceInfoBackend } from "./backend";
import { deviceInfoError, type DeviceInfoErrorCode } from "./errors";
import type { AppInfo, Model, OsVersion } from "./types";

/** The answers a memory backend gives. A code makes that query reject with it. */
export interface MemoryDeviceInfoConfig {
  model?: Model | DeviceInfoErrorCode;
  osVersion?: OsVersion | DeviceInfoErrorCode;
  appInfo?: AppInfo | DeviceInfoErrorCode;
}

/** A backend the test drives: scripted answers and a count of each query's reads. */
export interface MemoryBackend extends DeviceInfoBackend {
  setModel(answer: Model | DeviceInfoErrorCode): void;
  setOsVersion(answer: OsVersion | DeviceInfoErrorCode): void;
  setAppInfo(answer: AppInfo | DeviceInfoErrorCode): void;
  modelReads(): number;
  osVersionReads(): number;
  appInfoReads(): number;
}

/** A memory backend, for tests and for apps that run without the crate. Missing data reads as null. */
export function memoryBackend(config: MemoryDeviceInfoConfig = {}): MemoryBackend {
  let model: Model | DeviceInfoErrorCode = config.model ?? { identifier: null, name: null };
  let osVersion: OsVersion | DeviceInfoErrorCode = config.osVersion ?? { major: 0, minor: 0, full: null };
  let appInfo: AppInfo | DeviceInfoErrorCode = config.appInfo ?? {
    appName: "",
    packageName: "",
    version: "",
    buildNumber: "",
    buildSignature: null,
    installerStore: null,
  };
  let modelReads = 0;
  let osVersionReads = 0;
  let appInfoReads = 0;

  return {
    deviceModel: () => {
      modelReads += 1;
      return typeof model === "string" ? Promise.reject(deviceInfoError(model)) : Promise.resolve(model);
    },
    osVersion: () => {
      osVersionReads += 1;
      return typeof osVersion === "string" ? Promise.reject(deviceInfoError(osVersion)) : Promise.resolve(osVersion);
    },
    appInfo: () => {
      appInfoReads += 1;
      return typeof appInfo === "string" ? Promise.reject(deviceInfoError(appInfo)) : Promise.resolve(appInfo);
    },
    setModel: (next) => {
      model = next;
    },
    setOsVersion: (next) => {
      osVersion = next;
    },
    setAppInfo: (next) => {
      appInfo = next;
    },
    modelReads: () => modelReads,
    osVersionReads: () => osVersionReads,
    appInfoReads: () => appInfoReads,
  };
}
