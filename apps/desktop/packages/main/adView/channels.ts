// No imports: this file is bundled into two preloads and the main process.
export const AD_VIEW = {
  openWhyAds: "adView:openWhyAds", // ad page -> main
  openRemoveAds: "adView:openRemoveAds", // ad page -> main
  state: "adView:state", // main -> ad page, payload AdPageState
  hostOpenWhyAds: "adView:host:openWhyAds", // main -> main window
  hostOpenRemoveAds: "adView:host:openRemoveAds",
  setLabels: "adView:setLabels", // main window -> main, payload { whyAds: string }
  setRemoveAds: "adView:setRemoveAds", // main window -> main, payload string | null
  suppress: "adView:suppress" // main window preload -> main, { reason: "fatal", on: boolean }
} as const
