import { FEReleaseChannel } from "@gd/core_module/bindings"
import { BrowserWindow, ipcMain } from "electron"
import { autoUpdater } from "electron-updater"

console.log(autoUpdater.currentVersion)

export default function initAutoUpdater(win: BrowserWindow | null) {
  console.log("Initializing auto updater")
  autoUpdater.autoDownload = false
  autoUpdater.autoInstallOnAppQuit = true

  ipcMain.handle(
    "checkForUpdates",
    async (_, selectedChannel: FEReleaseChannel) => {
      console.log("Checking for updates", selectedChannel)
      if (__APP_VERSION__.includes("snapshot")) {
        return null
      }

      let selectedChannelNumber
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

      let currentChannelNumber
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
      console.log("Checking for updates", selectedChannel)
      console.log("Current version", autoUpdater.currentVersion)

      try {
        await autoUpdater.checkForUpdates()
      } catch (error) {
        console.error("Error checking for updates", error)
      }
    }
  )

  autoUpdater.on("update-available", (updateInfo) => {
    console.log("Update available", updateInfo)
    if (win) {
      win?.webContents.send("updateAvailable", updateInfo)
    }

    autoUpdater.downloadUpdate()
  })

  autoUpdater.on("update-not-available", () => {
    if (win) {
      win.webContents.send("updateNotAvailable")
    }
  })

  autoUpdater.on("download-progress", (progress) => {
    console.log("Download progress", progress)
    if (win) {
      win.webContents.send("downloadProgress", progress)
    }
  })

  ipcMain.handle("installUpdate", async () => {
    autoUpdater.quitAndInstall(true, true)
  })

  autoUpdater.on("update-downloaded", () => {
    console.log("Update downloaded, ready to install")
    if (win) {
      win?.webContents.send("updateDownloaded")
    }
  })
}
