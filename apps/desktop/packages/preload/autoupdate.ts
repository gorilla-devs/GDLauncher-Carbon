import { FEReleaseChannel } from "@gd/core_module/bindings"
import { contextBridge, ipcRenderer, IpcRendererEvent } from "electron"

import type { UpdateStateData } from "../main/utils/updateState"

contextBridge.exposeInMainWorld(
  "checkForUpdates",
  async (releaseChannel: FEReleaseChannel) =>
    ipcRenderer.invoke("checkForUpdates", releaseChannel)
)

contextBridge.exposeInMainWorld("installUpdate", async () =>
  ipcRenderer.invoke("installUpdate")
)

contextBridge.exposeInMainWorld("getUpdateState", async () =>
  ipcRenderer.invoke("getUpdateState")
)

contextBridge.exposeInMainWorld(
  "onUpdateStateChanged",
  (cb: (_ev: IpcRendererEvent, _state: UpdateStateData) => void) =>
    ipcRenderer.on("gd-update-state-changed", cb)
)
