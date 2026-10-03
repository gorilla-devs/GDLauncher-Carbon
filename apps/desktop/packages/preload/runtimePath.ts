import { contextBridge, ipcRenderer } from "electron"

contextBridge.exposeInMainWorld("getRuntimePath", async () =>
  ipcRenderer.invoke("getRuntimePath")
)

contextBridge.exposeInMainWorld("getInitialRuntimePath", async () =>
  ipcRenderer.invoke("getInitialRuntimePath")
)

contextBridge.exposeInMainWorld(
  "changeRuntimePath",
  async (newPath: string | null, switchOnly?: boolean) =>
    ipcRenderer.invoke("changeRuntimePath", newPath, switchOnly)
)

contextBridge.exposeInMainWorld(
  "takeRuntimeMoveNotice",
  async (): Promise<RuntimeMoveNotice | null> =>
    ipcRenderer.invoke("takeRuntimeMoveNotice")
)

contextBridge.exposeInMainWorld(
  "dismissRuntimeMoveNotice",
  async (): Promise<void> => ipcRenderer.invoke("dismissRuntimeMoveNotice")
)

contextBridge.exposeInMainWorld(
  "getUnavailableRuntimePath",
  async (): Promise<{ folder: string; current: string } | null> =>
    ipcRenderer.invoke("getUnavailableRuntimePath")
)

contextBridge.exposeInMainWorld(
  "keepDefaultRuntimePath",
  async (): Promise<void> => ipcRenderer.invoke("keepDefaultRuntimePath")
)

contextBridge.exposeInMainWorld("changeRuntimePathProgress", async (cb: any) =>
  ipcRenderer.on("changeRuntimePathProgress", cb)
)

contextBridge.exposeInMainWorld(
  "validateRuntimePath",
  async (newPath: string | null): Promise<RuntimePathValidation> =>
    ipcRenderer.invoke("validateRuntimePath", newPath)
)
