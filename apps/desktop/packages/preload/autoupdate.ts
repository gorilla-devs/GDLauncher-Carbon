import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("checkForUpdates", async () =>
  ipcRenderer.invoke("checkForUpdates")
);

contextBridge.exposeInMainWorld("installUpdate", async () =>
  ipcRenderer.invoke("installUpdate")
);

contextBridge.exposeInMainWorld("updateAvailable", async (cb: any) =>
  ipcRenderer.on("updateAvailable", cb)
);

contextBridge.exposeInMainWorld(
  "releaseChannel",
  async (releaseChannel: string) =>
    ipcRenderer.send("releaseChannel", releaseChannel)
);
