import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getCoreModuleStatus", async () =>
  ipcRenderer.invoke("getCoreModuleStatus")
);
