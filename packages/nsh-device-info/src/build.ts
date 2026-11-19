import type { Os, AppInfo } from "./types";
import { asDeviceInfoError } from "./errors";

/** What the runtime's `appInfo()` reports: the app as `nsh.json` and the build gave it. */
export interface BuildInfo {
  readonly id: string | null;
  readonly name: string | null;
  readonly version: string | null;
  readonly build: number | null;
}

/** The runtime's build metadata as an `AppInfo`. A missing key is the empty string, as on every platform. */
export function appInfoFromBuild(build: BuildInfo): AppInfo {
  return {
    appName: build.name ?? "",
    packageName: build.id ?? "",
    version: build.version ?? "",
    buildNumber: build.build === null ? "" : String(build.build),
    buildSignature: null,
    installerStore: null,
  };
}

// The crate has no app info source on web or desktop Linux; the runtime's `appInfo()` is the only one there.
const BUILD_SOURCE_OS: readonly Os[] = ["web", "linux"];

/**
 * The crate's app info, with the runtime's build metadata as the answer on web and desktop Linux
 * when the crate reports `ERR_DEVICE_INFO_NOT_AVAILABLE`. Any other failure passes through.
 */
export async function appInfoWithBuildFallback(
  native: () => Promise<AppInfo>,
  os: Os,
  build: () => BuildInfo,
): Promise<AppInfo> {
  try {
    return await native();
  } catch (error) {
    const coded = asDeviceInfoError(error);
    if (coded.code === "ERR_DEVICE_INFO_NOT_AVAILABLE" && BUILD_SOURCE_OS.includes(os)) {
      return appInfoFromBuild(build());
    }
    throw coded;
  }
}
