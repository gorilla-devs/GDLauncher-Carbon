import { BrowserWindow, app, ipcMain } from "electron";
import { autoUpdater } from "electron-updater";

export default function initAutoUpdater(win: BrowserWindow) {
  let allowUnstableReleases = false;

  autoUpdater.autoDownload = false;
  autoUpdater.allowDowngrade =
    !allowUnstableReleases && app.getVersion().includes("beta");
  autoUpdater.allowPrerelease = allowUnstableReleases;

  ipcMain.handle("checkUpdate", async () => {
    autoUpdater.checkForUpdates();
  });

  ipcMain.handle("installUpdate", async () => {
    // autoUpdater.quitAndInstall(true, false);
  });

  autoUpdater.on("update-available", () => {
    // autoUpdater.downloadUpdate();
  });

  autoUpdater.on("update-downloaded", () => {
    win?.webContents.send("updateAvailable");
  });

  ipcMain.on("releaseChannel", async (_, releaseChannel) => {
    if (releaseChannel === "beta" || releaseChannel === "alpha") {
      allowUnstableReleases = true;
    }
  });
}
