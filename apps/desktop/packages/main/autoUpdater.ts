import { assertRuntimeOperationAvailable } from "./utils/runtimeOperation"
import type { FEReleaseChannel } from "@gd/core_module/bindings"
import { BrowserWindow, ipcMain } from "electron"
import { autoUpdater } from "electron-updater"
import log from "electron-log/main"
import fs from "node:fs/promises"
import path from "node:path"
import { activeUpdateMetadata } from "./utils/updateProvider"
import type { Installation } from "./utils/updateArtifact"
import { createUpdaterController } from "./utils/updaterController"
import { ownsDownloadedUpdate } from "./utils/downloadedUpdate"
import { createOwnedInstallDispatch } from "./utils/ownedInstallDispatch"

async function installationIdentity(): Promise<Installation> {
  let format: Installation["format"] = null
  if (process.platform === "linux") {
    if (process.env.APPIMAGE) format = "appimage"
    else {
      try {
        const packageType = (
          await fs.readFile(
            path.join(process.resourcesPath, "package-type"),
            "utf8"
          )
        ).trim()
        if (packageType === "deb" || packageType === "rpm") format = packageType
        else
          return {
            platform: process.platform,
            arch: process.arch,
            format: null
          }
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT")
          return {
            platform: process.platform,
            arch: process.arch,
            format: null
          }
      }
      // This builder's other supported Linux format is ZIP.
      if (!format) format = "zip"
    }
  } else if (process.platform === "darwin") format = "zip"
  else if (process.platform === "win32" && process.env.PORTABLE_EXECUTABLE_DIR)
    format = "zip"
  return { platform: process.platform, arch: process.arch, format }
}
export default function initAutoUpdater(
  updateFeedOverride?: string | null,
  prepareShutdown?: () => Promise<"stopped" | "forced" | "kept">
) {
  log.transports.file.level = "info"
  autoUpdater.logger = log
  if (updateFeedOverride)
    autoUpdater.setFeedURL({ provider: "generic", url: updateFeedOverride })
  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = true
  autoUpdater.disableDifferentialDownload = true
  autoUpdater.disableWebInstaller = true
  let metadata: ReturnType<typeof activeUpdateMetadata> | undefined
  const metadataReady = installationIdentity().then((identity) => {
    metadata = activeUpdateMetadata(autoUpdater, identity, __APP_VERSION__)
  })
  const controller = createUpdaterController({
    version: __APP_VERSION__,
    updater: autoUpdater,
    dispatchInstall: createOwnedInstallDispatch(autoUpdater),
    isDownloaded: (info) => ownsDownloadedUpdate(autoUpdater, info),
    broadcast: (state) =>
      BrowserWindow.getAllWindows().forEach((window) => {
        if (!window.isDestroyed())
          window.webContents.send("gd-update-state-changed", state)
      }),
    artifact: (info) => metadata?.artifact(info) ?? null,
    manual: async () => {
      await metadataReady
      return metadata!.manual()
    },
    installBlocker: () => appImageInstallBlocker(process.env.APPIMAGE),
    prepareShutdown: async () => {
      assertRuntimeOperationAvailable()
      if (!prepareShutdown)
        throw new Error("Owned core shutdown is unavailable")
      return prepareShutdown()
    }
  })
  ipcMain.handle("checkForUpdates", async (_, channel: FEReleaseChannel) => {
    await metadataReady
    return controller.check(channel)
  })
  ipcMain.handle("installUpdate", (_, expectedId?: number) =>
    controller.install(expectedId)
  )
  ipcMain.handle("getUpdateState", () => controller.getState())
}
