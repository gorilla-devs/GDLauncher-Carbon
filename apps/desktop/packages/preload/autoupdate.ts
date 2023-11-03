import { FEReleaseChannel } from "@gd/core_module/bindings";
import { contextBridge, ipcRenderer, IpcRendererEvent } from "electron";
import type { ProgressInfo, UpdateInfo } from "electron-updater";

contextBridge.exposeInMainWorld(
  "checkForUpdates",
  async (releaseChannel: FEReleaseChannel) =>
    ipcRenderer.invoke("checkForUpdates", releaseChannel)
);

contextBridge.exposeInMainWorld("installUpdate", async () =>
  ipcRenderer.invoke("installUpdate")
);

contextBridge.exposeInMainWorld("downloadUpdate", async () =>
  ipcRenderer.invoke("downloadUpdate")
);

contextBridge.exposeInMainWorld(
  "onDownloadProgress",
  async (cb: (_ev: IpcRendererEvent, _progressInfo: ProgressInfo) => void) =>
    ipcRenderer.on("downloadProgress", cb)
);

contextBridge.exposeInMainWorld(
  "updateDownloaded",
  async (cb: (_ev: IpcRendererEvent) => void) =>
    ipcRenderer.on("updateDownloaded", cb)
);

contextBridge.exposeInMainWorld(
  "updateAvailable",
  async (cb: (_ev: IpcRendererEvent, _updateInfo: UpdateInfo) => void) =>
    ipcRenderer.on("updateAvailable", cb)
);

contextBridge.exposeInMainWorld(
  "updateNotAvailable",
  async (cb: (_ev: IpcRendererEvent) => void) =>
    ipcRenderer.on("updateNotAvailable", cb)
);
