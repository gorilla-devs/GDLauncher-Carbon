import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("changeRuntimePath", async (newPath: string) =>
  ipcRenderer.invoke("changeRuntimePath", newPath)
);
