import { contextBridge, ipcRenderer, webUtils } from "electron"
import { THEME_HOST } from "../main/themeChannels"

// Deliberately limited: the page can only provide menu labels, react to the
// reset request and ask where a dropped file is. Nothing here reaches the ad
// view.
contextBridge.exposeInMainWorld("themeHost", {
  // A dropped file's path comes from Electron, not from the page. The core
  // still checks the file before anything reaches the page.
  pathForFile(file: File): string {
    try {
      return webUtils.getPathForFile(file)
    } catch {
      return ""
    }
  },
  onResetTheme(cb: () => void) {
    const listener = () => cb()
    ipcRenderer.on(THEME_HOST.reset, listener)
    return () => {
      ipcRenderer.removeListener(THEME_HOST.reset, listener)
    }
  },
  onWindowVisibility(cb: (visible: boolean) => void) {
    const listener = (_e: unknown, visible: boolean) => cb(visible === true)
    ipcRenderer.on(THEME_HOST.windowVisibility, listener)
    return () => {
      ipcRenderer.removeListener(THEME_HOST.windowVisibility, listener)
    }
  },
  setMenuLabels(labels: { view: string; resetTheme: string }) {
    ipcRenderer.send(THEME_HOST.setMenuLabels, labels)
  }
})
