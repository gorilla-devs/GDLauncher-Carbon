import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("relaunch", () =>
  ipcRenderer.invoke("relaunch")
);
