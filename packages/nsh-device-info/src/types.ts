/**
 * The answers this library gives. Field names follow device_info_plus and package_info_plus.
 */

/** The device's hardware. Missing data is `null`, never an error. */
export interface Model {
  /** The raw hardware model string exactly as the platform reports it ("iPhone16,2", "SM-S928B"). */
  identifier: string | null;
  /** A marketing name, only when the platform itself provides one. No mapping table in v0.1. */
  name: string | null;
}

/** The OS the app runs on. `major`/`minor` are numbers; anything else stays in `full`. */
export interface OsVersion {
  major: number;
  minor: number;
  /** The platform's whole version string ("18.1"), when it gives one beyond major.minor. */
  full: string | null;
}

/** The running app's own identity, from its build metadata. A missing key is the empty string. */
export interface AppInfo {
  appName: string;
  packageName: string;
  version: string;
  buildNumber: string;
  buildSignature: string | null;
  installerStore: string | null;
}

/** The operating systems an NSH app runs on; the same values as the runtime's `platform.os`. */
export type Os = "ios" | "android" | "macos" | "windows" | "linux" | "web";
