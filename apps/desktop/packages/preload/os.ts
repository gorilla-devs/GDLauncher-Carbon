import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getCurrentOS", (os: any) => {
  return ipcRenderer.invoke("getCurrentOS", os);
});
