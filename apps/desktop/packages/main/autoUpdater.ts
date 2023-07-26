import { BrowserWindow, app, ipcMain } from "electron";
import { autoUpdater } from "electron-updater";

export default function initAutoUpdater(win: BrowserWindow) {
  const isUnstable =
    app.getVersion().includes("alpha") || app.getVersion().includes("beta");

  autoUpdater.autoDownload = false;
  autoUpdater.allowDowngrade = isUnstable;
  autoUpdater.allowPrerelease = true;

  ipcMain.handle("checkForUpdates", async () => {
    console.log("Checking for updates");
    return autoUpdater.checkForUpdates();
    // return true;
  });

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
