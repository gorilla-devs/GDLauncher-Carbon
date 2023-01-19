import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getAdSize", async () =>
  ipcRenderer.invoke("getAdSize")
);
contextBridge.exposeInMainWorld("adSizeChanged", async (cb: any) =>
  ipcRenderer.on("adSizeChanged", cb)
);
