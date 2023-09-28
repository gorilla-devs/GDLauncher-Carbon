import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getRuntimePath", async () =>
  ipcRenderer.invoke("getRuntimePath")
);

contextBridge.exposeInMainWorld("getInitialRuntimePath", async () =>
  ipcRenderer.invoke("getInitialRuntimePath")
);

contextBridge.exposeInMainWorld(
  "changeRuntimePath",
  async (newPath: string | null) =>
    ipcRenderer.invoke("changeRuntimePath", newPath)
);

contextBridge.exposeInMainWorld(
  "validateRuntimePath",
  async (newPath: string | null) =>
    ipcRenderer.invoke("validateRuntimePath", newPath)
);
