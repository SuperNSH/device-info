// Local runs are forbidden (AGENTS.md), so GitHub CI sees each assertion fail once.
import { expect, test as it } from "nsh/test";
import { DeviceInfoError, appInfoFromBuild, appInfoWithBuildFallback } from "../src";
import type { AppInfo } from "../src";

const build = { id: "dev.supernsh.notes", name: "Notes", version: "2.3.0", build: 42 };
const unavailable = async (): Promise<AppInfo> => {
  throw new DeviceInfoError("ERR_DEVICE_INFO_NOT_AVAILABLE");
};

it("build metadata becomes app info with the empty string for a missing key", () => {
  expect(appInfoFromBuild(build)).toEqual({
    appName: "Notes",
    packageName: "dev.supernsh.notes",
    version: "2.3.0",
    buildNumber: "42",
    buildSignature: null,
    installerStore: null,
  });
  expect(appInfoFromBuild({ id: null, name: null, version: null, build: null })).toEqual({
    appName: "",
    packageName: "",
    version: "",
    buildNumber: "",
    buildSignature: null,
    installerStore: null,
  });
});

it("web and linux answer app info from the build when the crate has no source", async () => {
  for (const os of ["web", "linux"] as const) {
    const info = await appInfoWithBuildFallback(unavailable, os, () => build);
    expect(info.version).toBe("2.3.0");
    expect(info.packageName).toBe("dev.supernsh.notes");
  }
});

it("a platform with a crate source never reads the build", async () => {
  const crate: AppInfo = { appName: "Crate", packageName: "p", version: "1", buildNumber: "7", buildSignature: null, installerStore: null };
  let reads = 0;
  const read = () => {
    reads += 1;
    return build;
  };
  expect(await appInfoWithBuildFallback(async () => crate, "web", read)).toEqual(crate);
  await expect(appInfoWithBuildFallback(unavailable, "macos", read)).rejects.toMatchObject({
    code: "ERR_DEVICE_INFO_NOT_AVAILABLE",
  });
  expect(reads).toBe(0);
});

it("a backend failure on web is not hidden by the build", async () => {
  await expect(
    appInfoWithBuildFallback(
      async () => {
        throw new DeviceInfoError("ERR_DEVICE_INFO_BACKEND");
      },
      "web",
      () => build,
    ),
  ).rejects.toMatchObject({ code: "ERR_DEVICE_INFO_BACKEND" });
});
