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
    adSize: { width: 440, height: 730, shouldShow: true },
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
  [1600, 900, 1260],
  [1599, 900, 1145],
  [1600, 899, 1145],
  [1400, 850, 1145],
  [1399, 850, 900],
  [1400, 849, 900],
  [1024, 700, 900],
  [1023, 700, 800],
  [1024, 699, 800]
])("tier threshold %s x %s stays %s", (width, height, minWidth) => {
  expect(getAdSize(display(width, height, 1)).minWidth).toBe(minWidth)
})
it("DIP fractions are retained", () => {
  expect(getAdSize(display(1024.5, 700.5, 2))).toMatchObject({
    width: 964.5,
    height: 660.5
  })
})
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
      adSize: { width: 0, height: 0, shouldShow: false },
      bannerAdSize: { width: 0, height: 0, shouldShow: false },
      hideAdText: true
    })
    expect(fixture.primary).not.toHaveBeenCalled()
  }
)
