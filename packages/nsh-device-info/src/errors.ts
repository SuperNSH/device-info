/** The codes a device-info rejection carries. Apps match on `code`. */
export type DeviceInfoErrorCode = "ERR_DEVICE_INFO_NOT_AVAILABLE" | "ERR_DEVICE_INFO_BACKEND";

/** Same wording as the Rust crate's messages. */
const MESSAGES: Record<DeviceInfoErrorCode, string> = {
  ERR_DEVICE_INFO_NOT_AVAILABLE: "Device information is not available on this platform.",
  ERR_DEVICE_INFO_BACKEND: "The platform's device information service failed.",
};

const KNOWN = new Set<string>(Object.keys(MESSAGES));

/** A rejection of the device-info API. `code` is the stable name; the message says what happened. */
export class DeviceInfoError extends Error {
  readonly code: DeviceInfoErrorCode;

  constructor(code: DeviceInfoErrorCode) {
    super(MESSAGES[code]);
    this.name = "DeviceInfoError";
    this.code = code;
  }
}

/** The error for a known code. */
export function deviceInfoError(code: DeviceInfoErrorCode): DeviceInfoError {
  return new DeviceInfoError(code);
}

/**
 * Turns whatever the native side or a host threw into a coded `DeviceInfoError`. A host's
 * rejection carries `ERR_NSH_HOST_ERROR`, and its message starts with the code, as
 * `ERR_DEVICE_INFO_BACKEND: text`. Anything unknown is a backend failure.
 */
export function asDeviceInfoError(error: unknown): DeviceInfoError {
  if (error instanceof DeviceInfoError) return error;
  const raw = (error ?? {}) as { code?: unknown; message?: unknown };
  const code = typeof raw.code === "string" ? raw.code : "";
  const message = typeof raw.message === "string" ? raw.message : "";
  if (code === "ERR_NSH_HOST_ERROR") {
    const named = /^(ERR_DEVICE_INFO_[A-Z_]+): /.exec(message)?.[1];
    return named !== undefined && KNOWN.has(named)
      ? deviceInfoError(named as DeviceInfoErrorCode)
      : deviceInfoError("ERR_DEVICE_INFO_BACKEND");
  }
  return KNOWN.has(code) ? deviceInfoError(code as DeviceInfoErrorCode) : deviceInfoError("ERR_DEVICE_INFO_BACKEND");
}
