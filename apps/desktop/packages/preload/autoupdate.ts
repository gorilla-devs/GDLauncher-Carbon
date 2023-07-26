import { FEReleaseChannel } from "@gd/core_module/bindings";
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld(
  "checkForUpdates",
  async (releaseChannel: string) =>
    ipcRenderer.invoke("checkForUpdates", releaseChannel)
);

contextBridge.exposeInMainWorld("installUpdate", async () =>
  ipcRenderer.invoke("installUpdate")
);

contextBridge.exposeInMainWorld("updateAvailable", async (cb: any) =>
  ipcRenderer.on("updateAvailable", cb)
);

contextBridge.exposeInMainWorld(
  "releaseChannel",
  async (releaseChannel: FEReleaseChannel) =>
    ipcRenderer.send("releaseChannel", releaseChannel)
);
