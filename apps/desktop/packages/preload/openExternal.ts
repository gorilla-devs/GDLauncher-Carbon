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

contextBridge.exposeInMainWorld("showSaveDialog", async (filters?: any) =>
  ipcRenderer.invoke("showSaveDialog", filters)
);

contextBridge.exposeInMainWorld("openFolder", async (link: string) =>
  ipcRenderer.invoke("openFolder", link)
);
