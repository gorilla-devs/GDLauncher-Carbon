import { BrowserWindow, app, ipcMain } from "electron";
import { autoUpdater } from "electron-updater";

export default function initAutoUpdater(win: BrowserWindow) {
  const isUnstable =
    app.getVersion().includes("alpha") || app.getVersion().includes("beta");

  autoUpdater.autoDownload = false;
  autoUpdater.allowDowngrade = isUnstable;
  autoUpdater.allowPrerelease = true;

  ipcMain.handle("checkUpdate", async () => {
    console.log("Checking for updates");
    autoUpdater.checkForUpdates();
  });

  ipcMain.handle("installUpdate", async () => {
    // autoUpdater.quitAndInstall(true, false);
  });

  autoUpdater.on("update-available", (updateInfo) => {
    console.log("Update available", updateInfo);
    // autoUpdater.downloadUpdate();
  });

  autoUpdater.on("update-not-available", (updateInfo) => {
    console.log("Update not available", updateInfo);
    // autoUpdater.downloadUpdate();
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
