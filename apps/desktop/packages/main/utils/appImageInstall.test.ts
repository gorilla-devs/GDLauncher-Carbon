import { afterEach, describe, expect, it } from "vitest"
import fs from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { appImageInstallBlocker } from "./appImageInstall"

const folders: string[] = []
afterEach(async () => {
  for (const folder of folders.splice(0)) {
    await fs.chmod(folder, 0o755).catch(() => {})
    await fs.rm(folder, { recursive: true, force: true })
  }
})
async function appImageIn(mode: number) {
  const folder = await fs.mkdtemp(path.join(os.tmpdir(), "gdl-appimage-"))
  folders.push(folder)
  const appImage = path.join(folder, "GDLauncher__2.0.40__linux__x64.AppImage")
  await fs.writeFile(appImage, "")
  await fs.chmod(folder, mode)
  return appImage
}
// root may write anywhere, so a read-only folder cannot be staged for it.
const asRoot = process.getuid?.() === 0

describe("an AppImage update install", () => {
  it("is not checked for anything but an AppImage", async () => {
    expect(await appImageInstallBlocker(undefined)).toBeNull()
  })
  it("may replace an AppImage in a folder the user can write to", async () => {
    expect(await appImageInstallBlocker(await appImageIn(0o755))).toBeNull()
  })
  it.skipIf(asRoot || process.platform === "win32")(
    "is refused for an AppImage in a folder the user cannot write to",
    async () => {
      expect(await appImageInstallBlocker(await appImageIn(0o555))).toBe(
        "install-folder-read-only"
      )
    }
  )
  it("leaves any other problem to the install itself", async () => {
    expect(
      await appImageInstallBlocker(
        path.join(os.tmpdir(), "gdl-missing-folder", "GDLauncher.AppImage")
      )
    ).toBeNull()
  })
})
