import path from "node:path"
import { describe, expect, it } from "vitest"
import { getBinaryPath, getCoreModulePath, getRootPath } from "./electronApp.js"

describe("getRootPath", () => {
  it("points at the unpacked build for each platform", () => {
    expect(
      getRootPath("win32").endsWith(path.join("release", "win-unpacked"))
    ).toBe(true)
    expect(
      getRootPath("linux").endsWith(path.join("release", "linux-unpacked"))
    ).toBe(true)
    expect(
      getRootPath("darwin").endsWith(
        path.join("release", "mac-universal", "GDLauncher.app")
      )
    ).toBe(true)
  })
})

describe("getBinaryPath", () => {
  it("names the executable for each platform", () => {
    expect(path.basename(getBinaryPath("win32"))).toBe("GDLauncher.exe")
    // electron-builder emits the linux binary under the package name.
    expect(path.basename(getBinaryPath("linux"))).toBe("@gddesktop")
    expect(path.basename(getBinaryPath("darwin"))).toBe("GDLauncher")
  })
})

describe("getCoreModulePath", () => {
  it("looks inside resources for each platform", () => {
    expect(getCoreModulePath("win32")).toContain(
      path.join("resources", "binaries", "core_module.exe")
    )
    expect(getCoreModulePath("linux")).toContain(
      path.join("resources", "binaries", "core_module")
    )
    // macOS keeps resources under Contents, not next to the executable.
    expect(getCoreModulePath("darwin")).toContain(
      path.join("Contents", "Resources", "binaries", "core_module")
    )
  })
})
