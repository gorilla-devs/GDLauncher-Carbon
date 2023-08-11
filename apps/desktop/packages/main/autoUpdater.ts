import { FEReleaseChannel } from "@gd/core_module/bindings";
import { BrowserWindow, ipcMain } from "electron";
import { autoUpdater } from "electron-updater";

export default function initAutoUpdater(win: BrowserWindow) {
  autoUpdater.autoDownload = false;

  ipcMain.handle(
    "checkForUpdates",
    async (_, selectedChannel: FEReleaseChannel) => {
      if (__APP_VERSION__.includes("snapshot")) {
        return null;
      }

      autoUpdater.channel =
        selectedChannel === "stable" ? "latest" : selectedChannel;
      autoUpdater.allowPrerelease = selectedChannel !== "stable";
      autoUpdater.allowDowngrade = selectedChannel !== "stable";
      console.log("Checking for updates", selectedChannel);
      console.log("Current version", autoUpdater.currentVersion);
      autoUpdater.checkForUpdates();
    }
  );

  ipcMain.handle("downloadUpdate", async () => {
    console.log("Downloading update");
    autoUpdater.downloadUpdate();
  });

  autoUpdater.on("update-available", (updateInfo) => {
    console.log("Update available", updateInfo);
    win.webContents.send("updateAvailable", updateInfo);
  });

  autoUpdater.on("update-not-available", () => {
    win.webContents.send("updateNotAvailable");
  });

  autoUpdater.on("download-progress", (progress) => {
    console.log("Download progress", progress);
    win.webContents.send("downloadProgress", progress);
  });

  ipcMain.handle("installUpdate", async () => {
    autoUpdater.quitAndInstall(true, true);
  });

  autoUpdater.on("update-downloaded", () => {
    win?.webContents.send("updateDownloaded");
  });
}
