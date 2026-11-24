// @vitest-environment jsdom
import { describe, it, expect } from "vitest"
import { createEffect, createRoot } from "solid-js"
import type { FEResolvedTheme, FEThemeLayout } from "@gd/core_module/bindings"
import { NAVBAR_HEIGHT } from "../../../../main/adView/geometry"
import {
  AD_TOP,
  LAYOUT_DEFAULTS,
  NAV_METRICS,
  adColumnSpacer,
  effectiveTileSize,
  layout,
  railWidth,
  resolveLayout,
  showsSearchStrip,
  type ResolvedLayout
} from "../layout"
import { setActiveTheme } from "../active"

const NONE: FEThemeLayout = {
  nav: null,
  panels: null,
  tileStyle: null,
  tileSize: null
}

const mk = (l: Partial<FEThemeLayout> = {}): FEResolvedTheme => ({
  id: "t",
  name: "T",
  author: "a",
  version: "1",
  description: null,
  names: {},
  descriptions: {},
  previewUrl: null,
  colors: {},
  fonts: [],
  tokens: { font: null, fontMono: null, radiusScale: null },
  css: null,
  background: null,
  decorations: [],
  effects: [],
  layout: { ...NONE, ...l },
  sounds: {
    launch: null,
    downloadComplete: null,
    click: null,
    notification: null,
    error: null
  },
  warnings: []
})

const RAIL: Partial<FEThemeLayout> = {
  nav: "rail",
  panels: false,
  tileStyle: "caption",
  tileSize: 4
}

describe("resolveLayout", () => {
  it("a theme without a layout gets the defaults", () => {
    expect(LAYOUT_DEFAULTS).toEqual({
      nav: "standard",
      panels: true,
      tileStyle: "overlay",
      tileSize: null
    })
    expect(resolveLayout(null)).toEqual(LAYOUT_DEFAULTS)
    expect(resolveLayout(undefined)).toEqual(LAYOUT_DEFAULTS)
    expect(resolveLayout({ layout: NONE })).toEqual(LAYOUT_DEFAULTS)
  })

  it("the theme's choices come through", () => {
    expect(resolveLayout({ layout: { ...NONE, ...RAIL } })).toEqual({
      nav: "rail",
      panels: false,
      tileStyle: "caption",
      tileSize: 4
    })
    expect(resolveLayout({ layout: { ...NONE, nav: "compact" } }).nav).toBe(
      "compact"
    )
    expect(resolveLayout({ layout: { ...NONE, nav: "sideways" } }).nav).toBe(
      "standard"
    )
  })

  // The bindings type these as plain strings and numbers, and built-ins never
  // pass through the core's checks: the renderer allow-lists them again.
  it("anything outside the allow-list falls back to the default", () => {
    const odd = {
      nav: "constructor",
      panels: "no",
      tileStyle: "polaroid",
      tileSize: 9
    } as unknown as FEThemeLayout
    expect(resolveLayout({ layout: odd })).toEqual(LAYOUT_DEFAULTS)
    for (const nav of ["Rail", " rail", "toString", ""])
      expect(resolveLayout({ layout: { ...NONE, nav } }).nav).toBe("standard")
    for (const tileStyle of ["Caption", "__proto__", ""])
      expect(resolveLayout({ layout: { ...NONE, tileStyle } }).tileStyle).toBe(
        "overlay"
      )
    for (const tileSize of [0, 6, 2.5, -1, NaN, Infinity])
      expect(
        resolveLayout({ layout: { ...NONE, tileSize } }).tileSize
      ).toBeNull()
    for (const tileSize of [1, 5])
      expect(resolveLayout({ layout: { ...NONE, tileSize } }).tileSize).toBe(
        tileSize
      )
    // only an explicit false turns the panels off
    expect(resolveLayout({ layout: { ...NONE, panels: true } }).panels).toBe(
      true
    )
    // a theme object from before the layout existed
    expect(
      resolveLayout({} as unknown as Pick<FEResolvedTheme, "layout">)
    ).toEqual(LAYOUT_DEFAULTS)
  })
})

describe("effectiveTileSize", () => {
  it("the user's tile size wins over the theme's", () => {
    expect(effectiveTileSize(3, true, 5)).toBe(3)
    expect(effectiveTileSize(3, false, 5)).toBe(5)
    expect(effectiveTileSize(3, undefined, 5)).toBe(5)
    expect(effectiveTileSize(3, false, null)).toBe(3)
    expect(effectiveTileSize(undefined, false, null)).toBe(2)
    expect(effectiveTileSize(undefined, true, 5)).toBe(2)
  })
})

describe("layout metrics", () => {
  it("the page's ad top matches main's", () => {
    expect(AD_TOP).toBe(NAVBAR_HEIGHT)
    expect(NAV_METRICS.standard.height).toBe(AD_TOP)
  })

  it("names the nav sizes", () => {
    expect(NAV_METRICS).toEqual({
      standard: { height: 60, logo: 36, search: 40 },
      compact: { height: 48, logo: 28, search: 32 },
      rail: { width: 64 }
    })
  })

  it("the rail takes its own width", () => {
    expect(railWidth("rail")).toBe(64)
    expect(railWidth("standard")).toBe(0)
    expect(railWidth("compact")).toBe(0)
  })

  it("the ad column's art lines up with the ad view", () => {
    expect(adColumnSpacer("standard")).toBe(0)
    expect(adColumnSpacer("compact")).toBe(12)
    expect(adColumnSpacer("rail")).toBe(60)
  })

  it("the rail shows the search bar only on search pages", () => {
    expect(showsSearchStrip("rail", "/search")).toBe(true)
    expect(showsSearchStrip("rail", "/search/mods")).toBe(true)
    expect(showsSearchStrip("rail", "/addon/1/curseforge")).toBe(true)
    expect(showsSearchStrip("rail", "/library")).toBe(false)
    expect(showsSearchStrip("standard", "/search")).toBe(false)
    // the same routes that keep the search box expanded
    expect(showsSearchStrip("compact", "/addon/1/curseforge")).toBe(false)
    expect(showsSearchStrip("rail", "/library/instance/1/addons")).toBe(false)
    expect(showsSearchStrip("rail", "/searching")).toBe(false)
  })
})

describe("layout()", () => {
  it("follows the committed theme, not a hover preview", () => {
    setActiveTheme(mk(RAIL))
    expect(layout()).toEqual(resolveLayout({ layout: { ...NONE, ...RAIL } }))
    // hovering a tile of a theme without a layout leaves the saved one
    setActiveTheme(mk(), { preview: true })
    expect(layout().nav).toBe("rail")
    expect(layout().tileSize).toBe(4)
    // the saved theme changing does move it
    setActiveTheme(mk())
    expect(layout()).toEqual(LAYOUT_DEFAULTS)
    // and hovering a rail theme while main is saved doesn't
    setActiveTheme(mk(RAIL), { preview: true })
    expect(layout()).toEqual(LAYOUT_DEFAULTS)
  })

  it("an equal layout coming back doesn't wake its readers", () => {
    setActiveTheme(mk(RAIL))
    const seen: ResolvedLayout[] = []
    const dispose = createRoot((d) => {
      createEffect(() => seen.push(layout()))
      return d
    })
    expect(seen).toHaveLength(1)
    const first = layout()

    // a hover's leave commits a new object with the same content
    setActiveTheme(mk(RAIL))
    setActiveTheme(mk(RAIL), { preview: true })
    setActiveTheme(mk(RAIL))
    expect(seen).toHaveLength(1)
    expect(layout()).toBe(first)

    // only a real change wakes them
    setActiveTheme(mk({ ...RAIL, tileSize: 3 }))
    expect(seen).toHaveLength(2)
    expect(seen[1].tileSize).toBe(3)

    // a missing layout and an explicit default one are the same layout
    setActiveTheme(mk())
    setActiveTheme(
      mk({
        nav: "standard",
        panels: true,
        tileStyle: "overlay",
        tileSize: null
      })
    )
    expect(seen).toHaveLength(3)
    dispose()
  })
})
