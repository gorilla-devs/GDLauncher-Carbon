import { FEReleaseChannel } from "@gd/core_module/bindings";
import { BrowserWindow, ipcMain } from "electron";
import { autoUpdater } from "electron-updater";

console.log(autoUpdater.currentVersion);

export default function initAutoUpdater(win: BrowserWindow | null) {
  autoUpdater.autoDownload = false;
  autoUpdater.autoInstallOnAppQuit = true;

  ipcMain.handle(
    "checkForUpdates",
    async (_, selectedChannel: FEReleaseChannel) => {
      if (__APP_VERSION__.includes("snapshot")) {
        return null;
      }

      let selectedChannelNumber;
      switch (selectedChannel) {
        case "stable":
          selectedChannelNumber = 0;
          break;
        case "beta":
          selectedChannelNumber = 1;
          break;
        case "alpha":
          selectedChannelNumber = 2;
          break;
      }

      let currentChannelNumber;
      if (__APP_VERSION__.includes("beta")) {
        currentChannelNumber = 1;
      } else if (__APP_VERSION__.includes("alpha")) {
        currentChannelNumber = 2;
      } else {
        currentChannelNumber = 0;
      }

      autoUpdater.channel =
        selectedChannel === "stable" ? "latest" : selectedChannel;
      autoUpdater.allowPrerelease = selectedChannel !== "stable";
      autoUpdater.allowDowngrade = selectedChannelNumber < currentChannelNumber;
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
    win?.webContents.send("updateAvailable", updateInfo);
    autoUpdater.downloadUpdate();
  });

  autoUpdater.on("update-not-available", () => {
    win?.webContents.send("updateNotAvailable");
  });

  autoUpdater.on("download-progress", (progress) => {
    console.log("Download progress", progress);
    win?.webContents.send("downloadProgress", progress);
  });

  ipcMain.handle("installUpdate", async () => {
    autoUpdater.quitAndInstall(true, true);
  });

  autoUpdater.on("update-downloaded", () => {
    console.log("Update downloaded, ready to install");
    win?.webContents.send("updateDownloaded");
  });
}
