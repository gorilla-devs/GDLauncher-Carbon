import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getCoreModule", async () =>
  ipcRenderer.invoke("getCoreModule")
);
