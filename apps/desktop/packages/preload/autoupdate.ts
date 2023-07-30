import { FEReleaseChannel } from "@gd/core_module/bindings";
import { contextBridge, ipcRenderer, IpcRendererEvent } from "electron";
import { ProgressInfo } from "electron-updater";

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
