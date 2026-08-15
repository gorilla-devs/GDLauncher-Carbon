import os from "os"
import path from "path"
import { describe, expect, it } from "vitest"
import { assertSafeRuntimeTarget } from "../runtimePathGuard.js"

describe("assertSafeRuntimeTarget", () => {
  it("accepts_plain_user_path", () => {
    const target = path.join(os.homedir(), "gdl-runtime")
    expect(() => assertSafeRuntimeTarget(target)).not.toThrow()
  })

  it("rejects_win32_verbatim_paths", () => {
    expect(() =>
      assertSafeRuntimeTarget("\\\\?\\C:\\Windows\\Temp\\gdl", null, "win32")
    ).toThrow()
  })

  it("rejects_unc_paths", () => {
    expect(() =>
      assertSafeRuntimeTarget("\\\\localhost\\C$\\gdl", null, "win32")
    ).toThrow()
  })

  it("rejects_win32_device_paths", () => {
    expect(() =>
      assertSafeRuntimeTarget("\\\\.\\PhysicalDrive0", null, "win32")
    ).toThrow()
  })

  it("does not reject verbatim-looking paths on non-win32 platforms", () => {
    // The bypass this guard closes is Windows-specific: on posix platforms a
    // leading double separator has no special filesystem meaning, so the
    // early rejection only applies when `platform === "win32"`.
    expect(() =>
      assertSafeRuntimeTarget("//localhost/C$/gdl", "/tmp/current", "linux")
    ).not.toThrow()
  })

  it("rejects a relative path", () => {
    expect(() => assertSafeRuntimeTarget("relative/gdl")).toThrow()
  })

  it("rejects a filesystem root", () => {
    const root = path.parse(process.cwd()).root
    expect(() => assertSafeRuntimeTarget(root)).toThrow()
  })

  it("rejects nesting with the current runtime path", () => {
    const current = path.join(os.homedir(), "gdl-runtime")
    const nested = path.join(current, "nested")
    expect(() => assertSafeRuntimeTarget(nested, current)).toThrow()
    expect(() => assertSafeRuntimeTarget(current, nested)).toThrow()
  })
})
