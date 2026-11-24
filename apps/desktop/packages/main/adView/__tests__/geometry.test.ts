import { describe, expect, it, vi } from "vitest"
import {
  computeAdStackBounds,
  isAdsRoute,
  NAVBAR_HEIGHT,
  SLOT_GAP,
  LINK_BLOCK
} from "../geometry.js"
import getAdSize from "../../adSize.js"
import {
  NAV_METRICS,
  adColumnSpacer,
  railWidth,
  type NavStyle
} from "../../../mainWindow/src/themes/layout.js"

// adSize.ts picks the tier from the display it is given; it only touches
// Electron for the primary display and the OS for the title bar height.
vi.mock("electron", () => ({ screen: {} }))
vi.mock("os", async (importOriginal) => {
  const os = await importOriginal<typeof import("os")>()
  return { ...os, default: { ...os, platform: () => "linux" } }
})
vi.stubGlobal("__SHOWCASE_MODE__", false)

const content = { width: 1600, height: 986 }
const highImpact = {
  adSize: { width: 440, height: 730, shouldShow: true },
  bannerAdSize: { width: 400, height: 60, shouldShow: true },
  hideAdText: false
}
const STACK = 60 + SLOT_GAP + 730 + SLOT_GAP + LINK_BLOCK

describe("computeAdStackBounds", () => {
  it("pins the stack to the right edge under the navbar", () => {
    expect(computeAdStackBounds(content, highImpact)).toEqual({
      x: 1160,
      y: 60,
      width: 440,
      height: STACK
    })
  })
  it("leaves out the banner and link when they are hidden", () => {
    const sky = {
      adSize: { width: 160, height: 600, shouldShow: true },
      bannerAdSize: { width: 0, height: 0, shouldShow: false },
      hideAdText: true
    }
    expect(computeAdStackBounds({ width: 1000, height: 720 }, sky)).toEqual({
      x: 840,
      y: 60,
      width: 160,
      height: 600
    })
  })
  it("never extends below the window", () => {
    expect(
      computeAdStackBounds({ width: 1600, height: 700 }, highImpact)?.height
    ).toBe(640)
  })
  it("returns null when ads are not shown", () => {
    expect(
      computeAdStackBounds(content, {
        adSize: { width: 0, height: 0, shouldShow: false }
      })
    ).toBeNull()
  })
  it("uses the given top", () => {
    expect(computeAdStackBounds(content, highImpact, 0)?.y).toBe(0)
  })
})

// The theme's layout menu never reaches main: the view's bounds are the same
// in every nav style, and the page lines its ad column up with them.
describe("every nav style at every ad tier", () => {
  // adSize.ts's Linux title bar: the window sizes it returns include it.
  const TITLEBAR = 30
  const TIERS = [
    { tier: 1, screen: { width: 1920, height: 1080 }, ad: 440, stack: STACK },
    {
      tier: 2,
      screen: { width: 1440, height: 900 },
      ad: 400,
      stack: 600 + SLOT_GAP + LINK_BLOCK
    },
    { tier: 3, screen: { width: 1280, height: 800 }, ad: 160, stack: 600 }
  ]
  const NAVS: NavStyle[] = ["standard", "compact", "rail"]
  const navHeight = (nav: NavStyle) =>
    nav === "rail" ? 0 : NAV_METRICS[nav].height

  it.each(TIERS)(
    "tier $tier: the ad keeps its full size, on screen, where the page reserves it",
    ({ screen, ad, stack }) => {
      const tier = getAdSize({
        workArea: { x: 0, y: 0, ...screen },
        scaleFactor: 1
      } as Electron.Display)
      expect(tier.adSize.width).toBe(ad)
      const windows = [
        { width: tier.width, height: tier.height - TITLEBAR },
        { width: tier.minWidth, height: tier.minHeight - TITLEBAR }
      ]
      for (const win of windows) {
        for (const nav of NAVS) {
          const where = `${win.width}x${win.height}, ${nav}`
          const b = computeAdStackBounds(win, tier)
          expect(b, where).toEqual({
            x: win.width - ad,
            y: NAVBAR_HEIGHT,
            width: ad,
            height: stack
          })
          expect(b!.y + b!.height, where).toBeLessThanOrEqual(win.height)
          // the column's first placeholder starts where the view does
          expect(navHeight(nav) + adColumnSpacer(nav), where).toBe(b!.y)
          // and the page still has room left of the column
          expect(win.width - railWidth(nav) - ad, where).toBeGreaterThan(0)
        }
      }
    }
  )
})

describe("isAdsRoute", () => {
  it.each([
    ["http://localhost:3344/#/library", true],
    [
      "file:///app/dist/mainWindow/index.html#/library?m%5B1%5D=onBoarding",
      true
    ],
    ["http://localhost:3344/#/settings/appearance", true],
    ["http://localhost:3344/#/", false],
    ["http://localhost:3344/#", false],
    ["http://localhost:3344/", false],
    ["file:///app/dist/mainWindow/index.html", false]
  ])("%s → %s", (url, expected) => {
    expect(isAdsRoute(url)).toBe(expected)
  })
})
