import { FEReleaseChannel } from "@gd/core_module/bindings";
import { BrowserWindow, ipcMain } from "electron";
import { autoUpdater } from "electron-updater";

export default function initAutoUpdater(win: BrowserWindow) {
  autoUpdater.autoDownload = false;
  autoUpdater.allowDowngrade = true;

  ipcMain.handle(
    "checkForUpdates",
    async (_, selectedChannel: FEReleaseChannel) => {
      if (__APP_VERSION__.includes("snapshot")) {
        return new Promise((r) => r(false));
      }
      autoUpdater.channel = selectedChannel;
      autoUpdater.allowPrerelease = selectedChannel !== "stable";
      console.log("Checking for updates", selectedChannel);
      return autoUpdater.checkForUpdates();
    }
  );

  ipcMain.handle("installUpdate", async () => {
    // autoUpdater.quitAndInstall(true, false);
  });

  autoUpdater.on("update-downloaded", () => {
    win?.webContents.send("updateAvailable");
  });

  // ipcMain.on("releaseChannel", async (_, releaseChannel) => {
  //   if (releaseChannel === "beta" || releaseChannel === "alpha") {
  //     allowUnstableReleases = true;
  //   }
  // });
}
