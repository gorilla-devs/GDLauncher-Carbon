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

// Task 4 component and platform controls.
it("rejects literal two-dot child in both directions", () => {
  expect(() =>
    assertSafeRuntimeTarget("/tmp/source/..nested", "/tmp/source", "linux")
  ).toThrow()
  expect(() =>
    assertSafeRuntimeTarget("/tmp/source", "/tmp/source/..nested", "linux")
  ).toThrow()
})
it("allows separate two-dot sibling", () => {
  expect(() =>
    assertSafeRuntimeTarget("/tmp/..nested", "/tmp/source", "linux")
  ).not.toThrow()
})
it("allows validated Linux removable descendants", () => {
  expect(() =>
    (assertSafeRuntimeTarget as any)(
      "/run/media/user/disk/gdl",
      "/tmp/source",
      "linux",
      {
        validatedRemovableMount: "/run/media/user/disk"
      }
    )
  ).not.toThrow()
})
it("uses Windows semantics for cross-drive destinations", () => {
  expect(() =>
    assertSafeRuntimeTarget("D:\\Games\\gdl", "C:\\Users\\me\\data", "win32")
  ).not.toThrow()
})

it.each([
  "/run",
  "/run/user/1000/gdl",
  "/run/media/user/disk/gdl",
  "/usr/gdl",
  "/"
])("rejects protected or unvalidated %s", (target) => {
  expect(() =>
    assertSafeRuntimeTarget(target, "/tmp/source", "linux")
  ).toThrow()
})
it.each([
  "D:\\",
  "C:\\Windows\\gdl",
  "C:\\Program Files\\gdl",
  "C:\\data\\..nested",
  "c:/DATA/child"
])("rejects dangerous Windows path %s", (target) => {
  expect(() => assertSafeRuntimeTarget(target, "C:\\data", "win32")).toThrow()
})
it("accepts /media descendants and Windows dot-segment sibling", () => {
  expect(() =>
    assertSafeRuntimeTarget("/media/user/disk/gdl", "/tmp/source", "linux")
  ).not.toThrow()
  expect(() =>
    assertSafeRuntimeTarget("D:/Games/../gdl", "C:\\data", "win32")
  ).not.toThrow()
})
it("checks the share a mapped drive resolves to, but never a typed network or verbatim path", () => {
  const resolved = { resolved: true, homePath: "C:\\Users\\me" }
  // Z:\GDL on a mapped drive resolves to its share.
  expect(() =>
    assertSafeRuntimeTarget(
      "\\\\nas\\share\\GDL",
      "C:\\Users\\me\\data",
      "win32",
      resolved
    )
  ).not.toThrow()
  // Two share paths still compare: nesting with a data folder on the same share.
  expect(() =>
    assertSafeRuntimeTarget(
      "\\\\nas\\share\\GDL\\sub",
      "\\\\NAS\\share\\GDL",
      "win32",
      resolved
    )
  ).toThrow(expect.objectContaining({ reason: "nested" }))
  // The share root is a drive root.
  expect(() =>
    assertSafeRuntimeTarget("\\\\nas\\share\\", null, "win32", resolved)
  ).toThrow(expect.objectContaining({ reason: "filesystem_root" }))
  for (const verbatim of ["\\\\?\\C:\\Windows\\gdl", "\\\\.\\PhysicalDrive0"])
    expect(() =>
      assertSafeRuntimeTarget(verbatim, null, "win32", resolved)
    ).toThrow(expect.objectContaining({ reason: "network_path" }))
  expect(() =>
    assertSafeRuntimeTarget("\\\\nas\\share\\GDL", null, "win32")
  ).toThrow(expect.objectContaining({ reason: "network_path" }))
})
it("lets a snapshot go back to its default data folder beside the executable, and nowhere else in its installation", () => {
  // A snapshot ZIP keeps its data in gdl_data beside the executable.
  const dir = "C:\\Users\\me\\Downloads\\GDLauncher"
  const windows = {
    homePath: "C:\\Users\\me",
    installationRoots: [dir, `${dir}\\resources`],
    defaultRuntimePath: `${dir}\\gdl_data\\data`
  }
  for (const target of [
    `${dir}\\gdl_data\\data`,
    "c:/users/ME/downloads/gdlauncher/GDL_DATA/data",
    `${dir}\\gdl_data\\data\\`,
    `${dir}\\locales\\..\\gdl_data\\data`
  ])
    expect(
      () => assertSafeRuntimeTarget(target, "D:\\GDL", "win32", windows),
      target
    ).not.toThrow()
  for (const target of [
    dir,
    `${dir}\\locales`,
    `${dir}\\resources`,
    `${dir}\\gdl_data`,
    `${dir}\\gdl_data\\data2`,
    `${dir}\\gdl_data\\data\\sub`,
    "C:\\Users\\me\\Downloads"
  ])
    expect(
      () => assertSafeRuntimeTarget(target, "D:\\GDL", "win32", windows),
      target
    ).toThrow(expect.objectContaining({ reason: "install_overlap" }))
  // The folder in use is still never a destination.
  expect(() =>
    assertSafeRuntimeTarget(
      `${dir}\\gdl_data\\data`,
      `${dir}\\gdl_data\\data`,
      "win32",
      windows
    )
  ).toThrow(expect.objectContaining({ reason: "nested" }))

  const linux = {
    homePath: "/home/me",
    installationRoots: ["/opt/GDLauncher", "/opt/GDLauncher/resources"],
    defaultRuntimePath: "/opt/GDLauncher/gdl_data/data"
  }
  for (const target of [
    "/opt/GDLauncher/gdl_data/data",
    "/opt/GDLauncher/gdl_data/data/"
  ])
    expect(
      () => assertSafeRuntimeTarget(target, "/mnt/GDL", "linux", linux),
      target
    ).not.toThrow()
  for (const target of [
    "/opt/GDLauncher",
    "/opt/GDLauncher/locales",
    "/opt/GDLauncher/gdl_data",
    "/opt/GDLauncher/gdl_data/data2",
    "/opt/GDLauncher/gdl_data/data/sub",
    "/opt"
  ])
    expect(
      () => assertSafeRuntimeTarget(target, "/mnt/GDL", "linux", linux),
      target
    ).toThrow(expect.objectContaining({ reason: "install_overlap" }))
})
it("keeps refusing the default data folder inside the installation when no default is given", () => {
  const dir = "C:\\Users\\me\\Downloads\\GDLauncher"
  expect(() =>
    assertSafeRuntimeTarget(`${dir}\\gdl_data\\data`, "D:\\GDL", "win32", {
      homePath: "C:\\Users\\me",
      installationRoots: [dir, `${dir}\\resources`]
    })
  ).toThrow(expect.objectContaining({ reason: "install_overlap" }))
  expect(() =>
    assertSafeRuntimeTarget(
      "/opt/GDLauncher/gdl_data/data",
      "/mnt/GDL",
      "linux",
      {
        homePath: "/home/me",
        installationRoots: ["/opt/GDLauncher", "/opt/GDLauncher/resources"]
      }
    )
  ).toThrow(expect.objectContaining({ reason: "install_overlap" }))
})
