import { it, expect, vi, beforeEach } from "vitest"
import type { Display } from "electron"
const fixture = vi.hoisted(() => ({ platform: "win32", primary: vi.fn() }))
vi.mock("electron", () => ({ screen: { getPrimaryDisplay: fixture.primary } }))
vi.mock("os", () => ({ default: { platform: () => fixture.platform } }))
import getAdSize from "../../adSize"
function display(width = 1920, height = 1040, scaleFactor = 2) {
  return {
    workArea: { width, height },
    size: { width: 600, height: 400 },
    scaleFactor
  } as Display
}
beforeEach(() => {
  fixture.platform = "win32"
  fixture.primary.mockReset()
  vi.stubGlobal("__SHOWCASE_MODE__", false)
})
it("Windows scaled DIP workArea keeps the large tier and exact titlebar geometry", () => {
  expect(getAdSize(display())).toEqual({
    minWidth: 1260,
    minHeight: 938,
    width: 1600,
    height: 988,
    adSize: {
      width: 440,
      height: 730,
      shouldShow: true,
      besideNavbar: false
    },
    bannerAdSize: { width: 400, height: 60, shouldShow: true },
    hideAdText: false
  })
})
it.each([
  ["win32", 1, 938, 988],
  ["linux", 2, 936, 986],
  ["darwin", 2, 934, 984]
])(
  "%s scale %s preserves its titlebar",
  (platform, scale, minHeight, height) => {
    fixture.platform = String(platform)
    expect(getAdSize(display(1920, 1040, Number(scale)))).toMatchObject({
      minWidth: 1260,
      minHeight,
      width: 1600,
      height
    })
  }
)
it.each([
  [1600, 938, 1260],
  // Both ads fit whole from 906 up, without the line of text under them.
  [1600, 906, 1260],
  // Too short for the large ad: the next tier, whose ad fits.
  [1600, 905, 1145],
  [1600, 900, 1145],
  [1599, 900, 1145],
  [1600, 899, 1145],
  [1400, 850, 1145],
  [1399, 850, 900],
  [1400, 849, 900],
  [1024, 702, 900],
  // Too short for the ad under the navbar: the ad column goes beside it.
  [1024, 701, 960],
  [1024, 700, 960],
  [1023, 700, 800],
  [1024, 699, 960],
  [1024, 642, 960],
  // Too short for the ad in either place: as it always was.
  [1024, 641, 800]
])("tier threshold %s x %s gives %s", (width, height, minWidth) => {
  expect(getAdSize(display(width, height, 1)).minWidth).toBe(minWidth)
})
it("DIP fractions are retained", () => {
  expect(getAdSize(display(1024.5, 760.5, 2))).toMatchObject({
    width: 964.5,
    height: 720.5
  })
})
it("a 1080p laptop at 150% gets the ad beside the navbar in a window that fits the screen", () => {
  // 1920x1080 at 150% is 1280x720 DIP; the taskbar leaves about 672.
  expect(getAdSize(display(1280, 672, 1.5))).toEqual({
    minWidth: 960,
    minHeight: 642,
    width: 1100,
    height: 672,
    adSize: { width: 160, height: 600, shouldShow: true, besideNavbar: true },
    bannerAdSize: { width: 0, height: 0, shouldShow: false },
    hideAdText: true
  })
})
it("1440p at 150% keeps both ads whole in a window that fits, without the line of text under them", () => {
  // 2560x1440 at 150% is about 1707x960 DIP; the taskbar leaves about 912.
  expect(getAdSize(display(1707, 912, 1.5))).toEqual({
    minWidth: 1260,
    minHeight: 906,
    width: 1600,
    height: 906,
    adSize: {
      width: 440,
      height: 730,
      shouldShow: true,
      besideNavbar: false
    },
    bannerAdSize: { width: 400, height: 60, shouldShow: true },
    hideAdText: true
  })
})
it("a display too short for the ad anywhere keeps the size it always had", () => {
  expect(getAdSize(display(1280, 600, 1))).toMatchObject({
    minWidth: 800,
    minHeight: 702,
    adSize: { width: 160, height: 600, besideNavbar: false }
  })
})
it.each([
  ["win32", 32],
  ["linux", 30],
  ["darwin", 28]
])(
  "on %s no display tall enough for an ad gets a window taller than itself or an ad that does not fit",
  (platform, titlebar) => {
    fixture.platform = platform
    const navbar = 60
    for (let width = 1024; width <= 3840; width += 64)
      for (let height = titlebar + 610; height <= 2160; height += 1) {
        const size = getAdSize(display(width, height, 1))
        const where = `${width}x${height}`
        expect(size.minHeight, where).toBeLessThanOrEqual(height)
        expect(size.minWidth, where).toBeLessThanOrEqual(width)
        expect(size.height, where).toBeGreaterThanOrEqual(size.minHeight)
        expect(size.height, where).toBeLessThanOrEqual(height)
        // The room the ad column has at the smallest window: under the
        // title bar, and under the navbar unless the column runs beside it.
        const column =
          size.minHeight - titlebar - (size.adSize.besideNavbar ? 0 : navbar)
        const ads =
          size.adSize.height +
          (size.bannerAdSize.shouldShow ? size.bannerAdSize.height + 8 : 0)
        expect(ads, where).toBeLessThanOrEqual(column)
      }
  }
)
it("primary lookup is used only without a supplied display and workArea wins over size", () => {
  fixture.primary.mockReturnValue(display())
  expect(getAdSize().minWidth).toBe(1260)
  expect(fixture.primary).toHaveBeenCalledOnce()
  getAdSize(display())
  expect(fixture.primary).toHaveBeenCalledOnce()
})
it.each(["win32", "linux", "darwin"])(
  "showcase bypasses display lookup on %s",
  (platform) => {
    fixture.platform = platform
    vi.stubGlobal("__SHOWCASE_MODE__", true)
    expect(getAdSize()).toEqual({
      minWidth: 1280,
      minHeight: 960,
      width: 1280,
      height: 960,
      adSize: { width: 0, height: 0, shouldShow: false, besideNavbar: false },
      bannerAdSize: { width: 0, height: 0, shouldShow: false },
      hideAdText: true
    })
    expect(fixture.primary).not.toHaveBeenCalled()
  }
)
