import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("openExternalLink", async (link: string) =>
  ipcRenderer.invoke("openExternalLink", link)
);
