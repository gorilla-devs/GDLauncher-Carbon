import { contextBridge, ipcRenderer } from "electron"

contextBridge.exposeInMainWorld("openCMPWindow", () => {
  return ipcRenderer.invoke("openCMPWindow")
})

contextBridge.exposeInMainWorld("isCMPWindowAvailable", () => {
  return ipcRenderer.invoke("isCMPWindowAvailable")
})
