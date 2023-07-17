import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("openExternalLink", async (link: string) =>
  ipcRenderer.invoke("openExternalLink", link)
);

contextBridge.exposeInMainWorld("copyToClipboard", async (link: string) =>
  ipcRenderer.invoke("copyToClipboard", link)
);

contextBridge.exposeInMainWorld("openFileDialog", async (filters?: any) =>
  ipcRenderer.invoke("openFileDialog", filters)
);
