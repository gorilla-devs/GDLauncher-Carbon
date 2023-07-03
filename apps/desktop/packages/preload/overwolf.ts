import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("openCMPWindow", () => {
  return ipcRenderer.invoke("openCMPWindow");
});
