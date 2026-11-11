// Every assertion fails a wrong implementation; local runs are forbidden (AGENTS.md), so GitHub CI sees each fail once.
// nsh/test has no it(); the pinned sentence names run under its test().
import { expect, test as it } from "nsh/test";
import { DeviceInfo, memoryBackend } from "../src";

it("each call reads the backend again", async () => {
  const backend = memoryBackend();
  backend.setModel({ identifier: "iPhone16,2", name: null });
  backend.setOsVersion({ major: 18, minor: 1, full: "18.1" });
  backend.setAppInfo({
    appName: "Notes",
    packageName: "dev.supernsh.notes",
    version: "1.0.0",
    buildNumber: "1",
    buildSignature: null,
    installerStore: null,
  });
  const info = new DeviceInfo(backend);

  expect(await info.deviceModel()).toEqual({ identifier: "iPhone16,2", name: null });
  expect((await info.osVersion()).major).toBe(18);
  expect((await info.appInfo()).version).toBe("1.0.0");

  // The answers change between calls; only a fresh read on every call can see the new ones.
  backend.setModel({ identifier: "iPhone17,1", name: null });
  backend.setOsVersion({ major: 19, minor: 0, full: "19.0" });
  backend.setAppInfo({
    appName: "Notes",
    packageName: "dev.supernsh.notes",
    version: "1.1.0",
    buildNumber: "2",
    buildSignature: null,
    installerStore: null,
  });

  expect(await info.deviceModel()).toEqual({ identifier: "iPhone17,1", name: null });
  expect(await info.osVersion()).toEqual({ major: 19, minor: 0, full: "19.0" });
  expect(await info.appInfo()).toEqual({
    appName: "Notes",
    packageName: "dev.supernsh.notes",
    version: "1.1.0",
    buildNumber: "2",
    buildSignature: null,
    installerStore: null,
  });

  expect(backend.modelReads()).toBe(2);
  expect(backend.osVersionReads()).toBe(2);
  expect(backend.appInfoReads()).toBe(2);
});

it("model keeps the raw identifier", async () => {
  const backend = memoryBackend();
  backend.setModel({ identifier: "iPhone16,2", name: null });
  const info = new DeviceInfo(backend);

  expect(await info.deviceModel()).toEqual({ identifier: "iPhone16,2", name: null });

  backend.setModel({ identifier: "SM-S928B", name: "Galaxy S24 Ultra" });
  expect(await info.deviceModel()).toEqual({
    identifier: "SM-S928B",
    name: "Galaxy S24 Ultra",
  });
});

it("partial data is null not an error", async () => {
  const backend = memoryBackend();
  backend.setModel({ identifier: null, name: null });
  backend.setOsVersion({ major: 0, minor: 0, full: null });
  backend.setAppInfo({
    appName: "",
    packageName: "",
    version: "",
    buildNumber: "",
    buildSignature: null,
    installerStore: null,
  });
  const info = new DeviceInfo(backend);

  expect(await info.deviceModel()).toEqual({ identifier: null, name: null });
  expect(await info.osVersion()).toEqual({ major: 0, minor: 0, full: null });
  expect(await info.appInfo()).toEqual({
    appName: "",
    packageName: "",
    version: "",
    buildNumber: "",
    buildSignature: null,
    installerStore: null,
  });
});

it("the facade passes through to its backend", async () => {
  const backend = memoryBackend();
  backend.setModel({ identifier: "Mac15,6", name: "Mac" });
  backend.setOsVersion({ major: 15, minor: 6, full: "15.6.1" });
  backend.setAppInfo({
    appName: "Notes",
    packageName: "dev.supernsh.notes",
    version: "2.3.4",
    buildNumber: "234",
    buildSignature: "a1b2c3",
    installerStore: "App Store",
  });
  const info = new DeviceInfo(backend);

  expect(await info.deviceModel()).toEqual({ identifier: "Mac15,6", name: "Mac" });
  expect(await info.osVersion()).toEqual({ major: 15, minor: 6, full: "15.6.1" });
  expect(await info.appInfo()).toEqual({
    appName: "Notes",
    packageName: "dev.supernsh.notes",
    version: "2.3.4",
    buildNumber: "234",
    buildSignature: "a1b2c3",
    installerStore: "App Store",
  });

  expect(backend.modelReads()).toBe(1);
  expect(backend.osVersionReads()).toBe(1);
  expect(backend.appInfoReads()).toBe(1);
});

it("unknown versions keep their string", async () => {
  const backend = memoryBackend();
  backend.setOsVersion({ major: 0, minor: 0, full: "nixos 25.05 (Quokka)" });
  const info = new DeviceInfo(backend);

  expect(await info.osVersion()).toEqual({ major: 0, minor: 0, full: "nixos 25.05 (Quokka)" });
});

it("os reports one of the six platforms", () => {
  const info = new DeviceInfo(memoryBackend());
  expect(["ios", "android", "macos", "windows", "linux", "web"]).toContain(info.os());
});
