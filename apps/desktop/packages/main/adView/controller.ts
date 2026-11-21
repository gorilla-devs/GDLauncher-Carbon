// Pure controller: turns inputs into commands on an AdViewPort. Must stay
// importable without Electron.
import {
  computeAdStackBounds,
  isAdsRoute,
  NAVBAR_HEIGHT,
  type AdLayout,
  type Rect,
  type Size
} from "./geometry"

export interface AdPageState {
  whyAds: string
  removeAds: string | null
}
// "shell" is internal: the main window page is not showing the ad layout yet.
export type SuppressReason = "fatal" | "adFree" | "shell"

export interface AdViewPort {
  setBounds(r: Rect): void
  setVisible(v: boolean): void
  // w, h, bw, bh, banner ("1"/"0"), hideText ("1"/"0")
  load(query: Record<string, string>): void
  sendState(s: AdPageState): void
}

export interface AdViewController {
  setLayout(l: AdLayout): void
  setContentSize(s: Size): void
  setRoute(url: string): void
  setSuppressed(reason: SuppressReason, on: boolean): void
  setLabels(whyAds: string): void
  setRemoveAds(label: string | null): void
  onPageReady(): void
}

function layoutQuery(l: AdLayout): Record<string, string> {
  const banner = l.bannerAdSize?.shouldShow === true
  return {
    w: String(l.adSize.width),
    h: String(l.adSize.height),
    bw: String(l.bannerAdSize?.width ?? 0),
    bh: String(l.bannerAdSize?.height ?? 0),
    banner: banner ? "1" : "0",
    hideText: l.hideAdText ? "1" : "0"
  }
}

function sameQuery(a: Record<string, string>, b: Record<string, string>) {
  const keys = Object.keys(a)
  return (
    keys.length === Object.keys(b).length && keys.every((k) => a[k] === b[k])
  )
}

function sameRect(a: Rect | null, b: Rect | null) {
  if (a === null || b === null) return a === b
  return (
    a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height
  )
}

export function createAdViewController(
  port: AdViewPort,
  init: { layout: AdLayout; contentSize: Size; top?: number }
): AdViewController {
  const top = init.top ?? NAVBAR_HEIGHT
  let layout = init.layout
  let content = init.contentSize
  let route = ""
  const suppressed = new Set<SuppressReason>(["shell"])
  let state: AdPageState = { whyAds: "Why are ads needed?", removeAds: null }

  let lastQuery = layoutQuery(layout)
  let lastBounds: Rect | null = null
  let lastVisible = false

  port.load(lastQuery)

  const sync = () => {
    const bounds = computeAdStackBounds(content, layout, top)
    if (bounds !== null && !sameRect(bounds, lastBounds)) {
      port.setBounds(bounds)
    }
    if (bounds !== null) lastBounds = bounds

    const visible =
      isAdsRoute(route) && suppressed.size === 0 && bounds !== null
    if (visible !== lastVisible) {
      lastVisible = visible
      port.setVisible(visible)
    }
  }

  sync()

  return {
    setLayout(l) {
      layout = l
      const q = layoutQuery(l)
      if (!sameQuery(q, lastQuery)) {
        lastQuery = q
        port.load(q)
      }
      sync()
    },
    setContentSize(s) {
      content = s
      sync()
    },
    setRoute(url) {
      route = url
      sync()
    },
    setSuppressed(reason, on) {
      if (on) suppressed.add(reason)
      else suppressed.delete(reason)
      sync()
    },
    setLabels(whyAds) {
      state = { ...state, whyAds }
      port.sendState(state)
    },
    setRemoveAds(label) {
      state = { ...state, removeAds: label }
      port.sendState(state)
    },
    onPageReady() {
      port.sendState(state)
    }
  }
}
