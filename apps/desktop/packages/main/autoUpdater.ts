import { FEReleaseChannel } from "@gd/core_module/bindings"
import { BrowserWindow, ipcMain } from "electron"
import { autoUpdater, UpdateInfo } from "electron-updater"
import log from "electron-log/main"

type UpdateState =
  | "idle"
  | "checking"
  | "downloading"
  | "downloaded"
  | "error"
  | "no-update"

interface UpdateStateData {
  state: UpdateState
  updateInfo: (UpdateInfo & { downloadUrl: string }) | null
  progress: number
  error: { message: string; details: string } | null
}

let autoUpdaterLock = false

let currentState: UpdateStateData = {
  state: "idle",
  updateInfo: null,
  progress: 0,
  error: null
}

function broadcastToAllWindows(channel: string, data?: any) {
  BrowserWindow.getAllWindows().forEach((window) => {
    if (!window.isDestroyed()) {
      window.webContents.send(channel, data)
    }
  })
}

function setUpdateState(newState: Partial<UpdateStateData>) {
  currentState = { ...currentState, ...newState }
  broadcastToAllWindows("gd-update-state-changed", currentState)
}

function releaseAutoUpdaterLock() {
  autoUpdaterLock = false
}

/**
 * Points the updater at `feedUrl` instead of the packaged `app-update.yml`.
 *
 * Only the e2e harness passes one (`--gdl_e2e_update_feed`), so it can serve
 * a channel file from its own mock rather than leaving every launch to fail
 * its update check against an unreachable release server. A failed check is
 * not inert: it raises an 8-second error toast over the login screen, which
 * an automated click then has to wait out.
 *
 * `provider: "generic"` matches what `.electron-builder.config.cjs` publishes
 * with, so the override changes where the channel file is fetched from and
 * nothing about how it is interpreted.
 */
function applyUpdateFeedOverride(feedUrl: string): void {
  autoUpdater.setFeedURL({ provider: "generic", url: feedUrl })
  log.info("[updater] Feed URL overridden:", feedUrl)
}

export default function initAutoUpdater(updateFeedOverride?: string | null) {
  log.transports.file.level = "info"
  autoUpdater.logger = log

  if (updateFeedOverride) {
    applyUpdateFeedOverride(updateFeedOverride)
  }
  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = true
  // Disable differential download - the CDN doesn't support HTTP range requests
  // (returns application/octet-stream instead of multipart/byteranges), which causes
  // the fallback to full download to not emit progress events properly
  autoUpdater.disableDifferentialDownload = true
  autoUpdater.disableWebInstaller = true

  ipcMain.handle(
    "checkForUpdates",
    async (_, selectedChannel: FEReleaseChannel) => {
      if (autoUpdaterLock) {
        return {
          status: "busy",
          currentState
        }
      }

      if (__APP_VERSION__.includes("snapshot")) {
        return {
          status: "snapshot-version",
          currentState
        }
      }

      autoUpdaterLock = true

      setUpdateState({
        state: "checking",
        error: null,
        progress: 0
      })

      const channelNumbers: Record<FEReleaseChannel, number> = {
        stable: 0,
        beta: 1,
        alpha: 2
      }
      const selectedChannelNumber = channelNumbers[selectedChannel]
      const currentChannelNumber = __APP_VERSION__.includes("alpha")
        ? 2
        : __APP_VERSION__.includes("beta")
          ? 1
          : 0

      autoUpdater.channel =
        selectedChannel === "stable" ? "latest" : selectedChannel
      autoUpdater.allowPrerelease = selectedChannel !== "stable"
      autoUpdater.allowDowngrade = selectedChannelNumber < currentChannelNumber

      try {
        await autoUpdater.checkForUpdates()
        return {
          status: "checking",
          currentState
        }
      } catch (error) {
        const errorDetails =
          error instanceof Error ? error.message : String(error)
        log.error("[updater] Error checking for updates:", errorDetails)
        log.error("[updater] Update URL:", autoUpdater.getFeedURL())
        releaseAutoUpdaterLock()
        setUpdateState({
          state: "error",
          error: {
            message: "Failed to check for updates",
            details: errorDetails
          }
        })
        return null
      }
    }
  )

  ipcMain.handle("installUpdate", async () => {
    autoUpdater.quitAndInstall(true, true)
  })

  ipcMain.handle("getUpdateState", () => {
    return currentState
  })

  autoUpdater.on("update-available", (updateInfo) => {
    // Don't release lock here - auto-download continues immediately
    // Lock will be released in downloaded/error handlers

    const baseUrl = process.env.GENERIC_PUBLISH_URL || "http://localhost:9000"
    const folder = process.env.PUBLISH_URL_FOLDER || ""
    const downloadUrl = `${baseUrl}${folder ? "/" + folder : ""}/${updateInfo.path}`

    setUpdateState({
      state: "downloading",
      progress: 0,
      updateInfo: { ...updateInfo, downloadUrl }
    })
  })

  autoUpdater.on("update-not-available", () => {
    releaseAutoUpdaterLock()
    setUpdateState({
      state: "no-update",
      updateInfo: null
    })
  })

  autoUpdater.on("download-progress", (progress) => {
    setUpdateState({ progress: progress.percent })
  })

  autoUpdater.on("update-downloaded", () => {
    releaseAutoUpdaterLock()

    setUpdateState({
      state: "downloaded",
      progress: 100
    })
  })

  autoUpdater.on("error", (error) => {
    log.error("[updater] Error:", error.message)
    log.error("[updater] Update URL:", autoUpdater.getFeedURL())

    releaseAutoUpdaterLock()

    setUpdateState({
      state: "error",
      error: {
        message: error.message || "Unknown error occurred",
        details: error.toString()
      }
    })
  })
}
