import { beforeEach, describe, expect, it } from "vitest"
import type { FEThemeDecoration } from "@gd/core_module/bindings"
import {
  cssLength,
  decorationStyle,
  CONTENT_COLUMN_CLIP,
  MODAL_ROW_CLIP,
  NAV_BAND_CLIP,
  hasDecorations,
  hashKey,
  pickVariant,
  resetCycleRegistry,
  slotClass,
  slotDecorations
} from "../decorations"

const deco = (over: Partial<FEThemeDecoration>): FEThemeDecoration => ({
  anchor: "library.tile",
  edge: "top",
  x: null,
  y: null,
  width: null,
  height: null,
  images: ["a.png"],
  pick: null,
  repeat: null,
  ...over
})

describe("pickVariant", () => {
  beforeEach(resetCycleRegistry)

  it("random picks are stable and depend only on the key", () => {
    const first = pickVariant("random", 16, "instance:7", 0, "library.tile")
    for (let i = 1; i <= 20; i++)
      pickVariant("random", 16, `instance:${i}`, 0, "library.tile")
    expect(pickVariant("random", 16, "instance:7", 0, "library.tile")).toBe(
      first
    )
    expect(pickVariant("random", 16, "instance:7", 0, "library.tile")).toBe(
      first
    )
    const used = new Set<number>()
    for (let i = 1; i <= 64; i++)
      used.add(pickVariant("random", 16, `instance:${i}`, 0, "library.tile"))
    expect(used.size).toBeGreaterThanOrEqual(12)
  })

  it("hashKey matches FNV-1a", () => {
    expect(hashKey("")).toBe(2166136261)
    expect(hashKey("a")).toBe(0xe40c292c)
  })

  it("cycle and first", () => {
    const seq = ["a", "b", "c", "d", "a"].map((k) =>
      pickVariant("cycle", 3, k, 0, "library.tile")
    )
    expect(seq).toEqual([0, 1, 2, 0, 0])
    expect(pickVariant("first", 3, "z", 0, "library.tile")).toBe(0)
    expect(pickVariant(null, 3, "z", 0, "library.tile")).toBe(0)
  })
})

describe("cssLength", () => {
  it("adds px to numbers and keeps strings", () => {
    expect(cssLength(12)).toBe("12px")
    expect(cssLength(-30)).toBe("-30px")
    expect(cssLength("16.3%")).toBe("16.3%")
    expect(cssLength(null)).toBeNull()
  })
})

describe("decorationStyle", () => {
  it("positions each edge", () => {
    const s = decorationStyle(
      deco({ x: "-3.26%", y: "-16.3%", width: "106.52%", height: "26.09%" }),
      "u.png"
    )
    expect(s).toMatchObject({
      position: "absolute",
      left: "-3.26%",
      top: "-16.3%",
      width: "106.52%",
      height: "26.09%"
    })
    const br = decorationStyle(
      deco({ edge: "bottom-right", x: 4, y: 6, width: 10, height: 10 }),
      "u.png"
    )
    expect(br.right).toBe("-4px")
    expect(br.bottom).toBe("-6px")
    expect(decorationStyle(deco({ edge: "left", width: 5 }), "u").height).toBe(
      "100%"
    )
    expect(
      decorationStyle(deco({ edge: "bottom", height: 5 }), "u").width
    ).toBe("100%")
  })

  it("repeats strips", () => {
    const x = decorationStyle(deco({ repeat: "x" }), "u")
    expect(x["background-size"]).toBe("auto 100%")
    expect(x["background-repeat"]).toBe("repeat-x")
    const y = decorationStyle(deco({ repeat: "y" }), "u")
    expect(y["background-size"]).toBe("100% auto")
    expect(y["background-repeat"]).toBe("repeat-y")
    expect(decorationStyle(deco({}), "u")["background-repeat"]).toBe(
      "no-repeat"
    )
  })

  it("escapes the url", () => {
    expect(decorationStyle(deco({}), 'a"b.png')["background-image"]).toBe(
      'url("a\\22 b.png")'
    )
  })
})

describe("slotClass", () => {
  it("clips nav, content and modal slots to their own anchor box", () => {
    // A tall `edge: bottom` nav decoration is placed from the nav's bottom
    // edge, so only the slot's own overflow keeps it out of the ad column.
    const tall = decorationStyle(
      deco({ anchor: "nav", edge: "bottom", height: 900 }),
      "u"
    )
    expect(tall.height).toBe("900px")
    for (const a of ["nav", "content", "modal"])
      expect(slotClass(a).split(" ")).toContain("[overflow:clip]")
    expect(slotClass("season-dialog").split(" ")).not.toContain(
      "[overflow:clip]"
    )
    expect(slotClass("panel").split(" ")).not.toContain("[overflow:clip]")
    expect(slotClass("nav")).toContain("pointer-events-none")
  })
})

describe("column and band clips", () => {
  it("clips the nav band and the content column's x", () => {
    expect(NAV_BAND_CLIP.overflow).toBe("clip")
    expect(CONTENT_COLUMN_CLIP["overflow-x"]).toBe("clip")
  })

  it("clips the nav band at its own box, so nothing reaches the ad view", () => {
    // The ad view starts right under the standard bar (y 60); any margin
    // would let a nav decoration paint its top rows.
    expect(NAV_BAND_CLIP).toEqual({ overflow: "clip" })
  })

  it("clips a dialog row sideways only, so a season cap still hangs over the top", () => {
    // The row's left part ends where the ad column starts; `clip`, unlike
    // `hidden`, leaves the other axis visible.
    expect(MODAL_ROW_CLIP).toEqual({ "overflow-x": "clip" })
  })
})

describe("more edges", () => {
  it("places right, top-right and bottom-left", () => {
    const r = decorationStyle(
      deco({ edge: "right", x: 3, y: 2, width: 8 }),
      "u"
    )
    expect(r).toMatchObject({
      right: "-3px",
      top: "2px",
      width: "8px",
      height: "100%"
    })
    expect(r.left).toBeUndefined()
    const tr = decorationStyle(
      deco({ edge: "top-right", x: "-5%", y: 1, width: 4, height: 4 }),
      "u"
    )
    expect(tr).toMatchObject({
      right: "5%",
      top: "1px",
      width: "4px",
      height: "4px"
    })
    const bl = decorationStyle(
      deco({ edge: "bottom-left", x: 7, y: -2, width: 4, height: 4 }),
      "u"
    )
    expect(bl).toMatchObject({ left: "7px", bottom: "2px" })
    expect(bl.top).toBeUndefined()
  })
})

describe("random spread", () => {
  it("does not alternate for power-of-two counts", () => {
    for (const count of [2, 4]) {
      const seq = Array.from({ length: 24 }, (_, i) =>
        pickVariant("random", count, `instance:${i + 1}`, 0, "library.tile")
      )
      expect(new Set(seq).size).toBe(count)
      // sequential ids must not walk through the variants in a strict pattern
      let alt = 1
      let maxAlt = 1
      let step = 1
      let maxStep = 1
      for (let i = 1; i < seq.length; i++) {
        alt = seq[i] !== seq[i - 1] ? alt + 1 : 1
        step = seq[i] === (seq[i - 1] + 1) % count ? step + 1 : 1
        maxAlt = Math.max(maxAlt, alt)
        maxStep = Math.max(maxStep, step)
      }
      expect(maxStep).toBeLessThan(count === 2 ? 8 : 6)
      if (count === 2) expect(maxAlt).toBeLessThan(8)
    }
  })
})

describe("slotDecorations", () => {
  beforeEach(resetCycleRegistry)
  it("filters by anchor, skips empty images and keeps full-array indices", () => {
    const theme = {
      decorations: [
        deco({ anchor: "nav", images: ["n.png"] }),
        deco({ anchor: "library.tile", images: [] }),
        deco({
          anchor: "library.tile",
          images: ["a.png", "b.png"],
          pick: "random"
        })
      ]
    }
    const out = slotDecorations(theme, "library.tile", "instance:3")
    expect(out).toHaveLength(1)
    expect(out[0].index).toBe(2)
    const v = pickVariant("random", 2, "instance:3", 2, "library.tile")
    expect(out[0].style["background-image"]).toBe(
      `url("${["a.png", "b.png"][v]}")`
    )
    expect(slotDecorations(null, "nav", "nav")).toEqual([])
    expect(slotDecorations(theme, "modal", "modal")).toEqual([])
  })

  it("new anchors get their own decorations", () => {
    const theme = {
      decorations: [
        deco({ anchor: "panel", images: ["p.png"] }),
        deco({
          anchor: "library.group",
          images: ["a.png", "b.png"],
          pick: "random"
        })
      ]
    }
    const out = slotDecorations(theme, "library.group", "group:12")
    expect(out).toHaveLength(1)
    expect(out[0].index).toBe(1)
    const v = pickVariant("random", 2, "group:12", 1, "library.group")
    expect(out[0].style["background-image"]).toBe(
      `url("${["a.png", "b.png"][v]}")`
    )
    expect(slotDecorations(theme, "section-title", "group:12")).toEqual([])
  })

  it("hasDecorations is true only when the theme decorates that anchor", () => {
    const theme = {
      decorations: [
        deco({ anchor: "library.group", images: ["a.png"] }),
        deco({ anchor: "page-title", images: [] })
      ]
    }
    expect(hasDecorations(theme, "library.group", "group:1")).toBe(true)
    expect(hasDecorations(theme, "page-title", "page-title")).toBe(false)
    expect(hasDecorations(theme, "section-title", "group:1")).toBe(false)
    expect(hasDecorations(null, "library.group", "group:1")).toBe(false)
  })
})
