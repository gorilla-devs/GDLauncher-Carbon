import { contextBridge, ipcRenderer } from "electron"
import { AD_VIEW } from "../main/adView/channels"

// Only available to the ad view's own page. Nothing here can show or hide ads.
contextBridge.exposeInMainWorld("gdlAdView", {
  openWhyAds: () => ipcRenderer.send(AD_VIEW.openWhyAds),
  openRemoveAds: () => ipcRenderer.send(AD_VIEW.openRemoveAds),
  onState: (cb: (s: { whyAds: string; removeAds: string | null }) => void) => {
    ipcRenderer.on(AD_VIEW.state, (_e, s) => cb(s))
  }
})
