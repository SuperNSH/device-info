// Every assertion fails a wrong implementation; local runs are forbidden (AGENTS.md), so GitHub CI sees each fail once.
// nsh/test has no it(); the pinned sentence names run under its test().
import { expect, test as it } from "nsh/test";
import { DeviceInfo, asDeviceInfoError, memoryBackend, DeviceInfoError } from "../src";

it("an unsupported target reports not available", async () => {
  const backend = memoryBackend();
  backend.setModel("ERR_DEVICE_INFO_NOT_AVAILABLE");
  backend.setOsVersion("ERR_DEVICE_INFO_NOT_AVAILABLE");
  backend.setAppInfo("ERR_DEVICE_INFO_NOT_AVAILABLE");
  const info = new DeviceInfo(backend);

  await expect(info.deviceModel()).rejects.toMatchObject({
    code: "ERR_DEVICE_INFO_NOT_AVAILABLE",
  });
  await expect(info.osVersion()).rejects.toMatchObject({
    code: "ERR_DEVICE_INFO_NOT_AVAILABLE",
  });
  await expect(info.appInfo()).rejects.toMatchObject({
    code: "ERR_DEVICE_INFO_NOT_AVAILABLE",
  });
});

it("a platform failure carries the backend code and status", async () => {
  // The JS bridge carries the code and the message; the numeric status is the Rust error's own field.
  const backend = memoryBackend();
  backend.setModel("ERR_DEVICE_INFO_BACKEND");
  backend.setOsVersion("ERR_DEVICE_INFO_BACKEND");
  backend.setAppInfo("ERR_DEVICE_INFO_BACKEND");
  const info = new DeviceInfo(backend);

  await expect(info.deviceModel()).rejects.toMatchObject({
    code: "ERR_DEVICE_INFO_BACKEND",
    message: "The platform's device information service failed.",
  });
  await expect(info.osVersion()).rejects.toMatchObject({ code: "ERR_DEVICE_INFO_BACKEND" });
  await expect(info.appInfo()).rejects.toMatchObject({ code: "ERR_DEVICE_INFO_BACKEND" });
});

it("every kind maps to its stable code", () => {
  const notAvailable = new DeviceInfoError("ERR_DEVICE_INFO_NOT_AVAILABLE");
  expect(notAvailable.code).toBe("ERR_DEVICE_INFO_NOT_AVAILABLE");
  expect(notAvailable.message).toBe("Device information is not available on this platform.");

  const backendFailure = new DeviceInfoError("ERR_DEVICE_INFO_BACKEND");
  expect(backendFailure.code).toBe("ERR_DEVICE_INFO_BACKEND");
  expect(backendFailure.message).toBe("The platform's device information service failed.");

  expect(asDeviceInfoError(new Error("boom")).code).toBe("ERR_DEVICE_INFO_BACKEND");
  const host = asDeviceInfoError({
    code: "ERR_NSH_HOST_ERROR",
    message: "ERR_DEVICE_INFO_NOT_AVAILABLE: x",
  });
  expect(host.code).toBe("ERR_DEVICE_INFO_NOT_AVAILABLE");
});
