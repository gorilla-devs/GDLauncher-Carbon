// Pure helpers: must stay importable without Electron.

export interface AdSlot {
  width: number
  height: number
  shouldShow: boolean
}
export interface AdLayout {
  adSize: AdSlot
  bannerAdSize?: AdSlot
  hideAdText?: boolean
}
export interface Size {
  width: number
  height: number
}
export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export const NAVBAR_HEIGHT = 60
// px between banner, main slot and link (today's `gap-2`)
export const SLOT_GAP = 8
// px reserved for the link pill
export const LINK_BLOCK = 34

export function computeAdStackBounds(
  content: Size,
  layout: AdLayout,
  top: number = NAVBAR_HEIGHT
): Rect | null {
  const { adSize, bannerAdSize, hideAdText } = layout
  if (!adSize.shouldShow) return null

  const banner = bannerAdSize?.shouldShow ? bannerAdSize.height + SLOT_GAP : 0
  const link = hideAdText ? 0 : SLOT_GAP + LINK_BLOCK
  const height = Math.min(
    banner + adSize.height + link,
    Math.max(0, content.height - top)
  )

  return {
    x: content.width - adSize.width,
    y: top,
    width: adSize.width,
    height
  }
}

export function isAdsRoute(url: string): boolean {
  const hashIndex = url.indexOf("#")
  if (hashIndex === -1) return false
  const path = url.slice(hashIndex + 1).split("?")[0]
  return path !== "" && path !== "/"
}
