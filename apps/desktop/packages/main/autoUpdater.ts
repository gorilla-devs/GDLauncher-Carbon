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

let isInstallingUpdate = false

export function installPendingUpdateOnQuit(): boolean {
  // If already installing, DON'T prevent quit - let quitAndInstall complete its own quit
  if (isInstallingUpdate) {
    return false
  }

  if (currentState.state === "downloaded") {
    isInstallingUpdate = true
    log.info("[updater] Installing pending update on quit...")
    try {
      autoUpdater.quitAndInstall(true, false)
      return true
    } catch (error) {
      log.error("[updater] Failed to install update on quit:", error)
      isInstallingUpdate = false
    }
  }
  return false
}

export default function initAutoUpdater(_win: BrowserWindow | null) {
  log.transports.file.level = "info"
  autoUpdater.logger = log
  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = false

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

      let selectedChannelNumber: number
      switch (selectedChannel) {
        case "stable":
          selectedChannelNumber = 0
          break
        case "beta":
          selectedChannelNumber = 1
          break
        case "alpha":
          selectedChannelNumber = 2
          break
      }

      let currentChannelNumber: number
      if (__APP_VERSION__.includes("beta")) {
        currentChannelNumber = 1
      } else if (__APP_VERSION__.includes("alpha")) {
        currentChannelNumber = 2
      } else {
        currentChannelNumber = 0
      }

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
