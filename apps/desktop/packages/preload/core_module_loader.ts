import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getCoreModulePort", async () =>
  ipcRenderer.invoke("getCoreModulePort")
);
