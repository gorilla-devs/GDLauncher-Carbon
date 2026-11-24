// Pure helpers: must stay importable without Electron.

export interface AdSlot {
  width: number
  height: number
  shouldShow: boolean
  /** The display is too short for the ad under the top bar, so the ad runs
   *  beside the bar, from the window's top (`adSize.ts`). */
  besideNavbar?: boolean
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

/**
 * Where the ad column starts: under the top bar, or at the window's top on a
 * display too short for that, where the column runs beside the bar.
 */
export function adColumnTop(
  layout: AdLayout,
  navTop: number = NAVBAR_HEIGHT
): number {
  return layout.adSize.besideNavbar ? 0 : navTop
}

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
