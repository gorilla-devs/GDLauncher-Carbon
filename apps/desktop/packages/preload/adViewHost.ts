import { contextBridge, ipcRenderer } from "electron"
import { AD_VIEW } from "../main/adView/channels"

// Deliberately limited: the page can only provide labels and react to clicks.
// There is no way to show or hide the ad view from the renderer.
function subscribe(channel: string, cb: () => void) {
  const listener = () => cb()
  ipcRenderer.on(channel, listener)
  return () => {
    ipcRenderer.removeListener(channel, listener)
  }
}

contextBridge.exposeInMainWorld("adView", {
  setLabels(whyAds: string) {
    ipcRenderer.send(AD_VIEW.setLabels, { whyAds })
  },
  setRemoveAds(label: string | null) {
    ipcRenderer.send(AD_VIEW.setRemoveAds, label)
  },
  onOpenWhyAds(cb: () => void) {
    return subscribe(AD_VIEW.hostOpenWhyAds, cb)
  },
  onOpenRemoveAds(cb: () => void) {
    return subscribe(AD_VIEW.hostOpenRemoveAds, cb)
  }
})
