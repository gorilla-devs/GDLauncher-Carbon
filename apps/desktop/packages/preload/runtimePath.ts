import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("getRuntimePath", async () =>
  ipcRenderer.invoke("getRuntimePath")
);

contextBridge.exposeInMainWorld("getInitialRuntimePath", async () =>
  ipcRenderer.invoke("getInitialRuntimePath")
);

contextBridge.exposeInMainWorld(
  "changeRuntimePath",
  async (newPath: string | null, cb: (progress: number) => void) =>
    ipcRenderer.invoke("changeRuntimePath", newPath, cb)
);

contextBridge.exposeInMainWorld("changeRuntimePathProgress", async (cb: any) =>
  ipcRenderer.on("changeRuntimePathProgress", cb)
);

contextBridge.exposeInMainWorld(
  "validateRuntimePath",
  async (newPath: string | null) =>
    ipcRenderer.invoke("validateRuntimePath", newPath)
);
