import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getMinimumBounds", async () =>
  ipcRenderer.invoke("getMinimumBounds")
);
contextBridge.exposeInMainWorld("minimumBoundsChanged", async (cb: any) =>
  ipcRenderer.on("minimumBoundsChanged", cb)
);
