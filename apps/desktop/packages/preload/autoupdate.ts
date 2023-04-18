import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("checkUpdate", async () =>
  ipcRenderer.invoke("checkUpdate")
);

contextBridge.exposeInMainWorld("installUpdate", async () =>
  ipcRenderer.invoke("installUpdate")
);

contextBridge.exposeInMainWorld("updateAvailable", async (cb: any) =>
  ipcRenderer.on("updateAvailable", cb)
);

// contextBridge.exposeInMainWorld(
//   "releaseChannel",
//   async (releaseChannel: string) =>
//     ipcRenderer.invoke("releaseChannel", releaseChannel)
// );
