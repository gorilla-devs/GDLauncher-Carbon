// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest"
import type { FEResolvedTheme } from "@gd/core_module/bindings"

class FakeFontFace {
  static instances: FakeFontFace[] = []
  loads = 0
  loadPromise: Promise<unknown> = Promise.resolve()
  constructor(
    public family: string,
    public source: string,
    public descriptors: Record<string, string>
  ) {
    FakeFontFace.instances.push(this)
  }
  load() {
    this.loads++
    return this.loadPromise
  }
}

const mk = (over: Partial<FEResolvedTheme> = {}): FEResolvedTheme => ({
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
  layout: { nav: null, panels: null, tileStyle: null, tileSize: null },
  sounds: {
    launch: null,
    downloadComplete: null,
    click: null,
    notification: null,
    error: null
  },
  warnings: [],
  ...over
})

const base = mk({
  colors: { primary: "9 9 9", "darkSlate-800": "10 20 30" },
  tokens: { font: "Ubuntu, sans-serif", fontMono: "Mono", radiusScale: null }
})

const font = (family: string, url: string) => ({
  family,
  url,
  weight: null,
  style: null
})

let fonts: Set<FakeFontFace>

async function load() {
  vi.resetModules()
  return await import("../apply")
}

beforeEach(() => {
  FakeFontFace.instances = []
  fonts = new Set()
  vi.stubGlobal("FontFace", FakeFontFace)
  Object.defineProperty(document, "fonts", {
    configurable: true,
    value: {
      add: (f: FakeFontFace) => fonts.add(f),
      delete: (f: FakeFontFace) => fonts.delete(f),
      has: (f: FakeFontFace) => fonts.has(f)
    }
  })
  document.head.innerHTML = "<style>.app{}</style>"
  document.body.innerHTML = ""
  document.documentElement.removeAttribute("style")
  delete document.documentElement.dataset.gdlTheme
  delete document.documentElement.dataset.gdlNav
  delete document.documentElement.dataset.gdlPanels
  delete document.documentElement.dataset.gdlTiles
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

const root = () => document.documentElement

describe("applyResolvedTheme", () => {
  it("sets every palette variable, filling gaps from the base", async () => {
    const { applyResolvedTheme } = await load()
    await applyResolvedTheme("a", mk({ colors: { primary: "1 2 3" } }), base)
    expect(root().style.getPropertyValue("--primary")).toBe("1 2 3")
    expect(root().style.getPropertyValue("--darkSlate-800")).toBe("10 20 30")
  })

  it("sets how light the main text colour is, from the theme or the base", async () => {
    const { applyResolvedTheme, textLightness } = await load()
    expect(textLightness("255 255 255")).toBe(1)
    expect(textLightness("80 80 80")).toBe(0.314)
    expect(textLightness("1 2")).toBeNull()
    expect(textLightness(undefined)).toBeNull()
    await applyResolvedTheme(
      "a",
      mk({ colors: { "lightSlate-50": "80 80 80" } }),
      base
    )
    expect(root().style.getPropertyValue("--gdl-text-lightness")).toBe("0.314")
    await applyResolvedTheme(
      "b",
      mk(),
      mk({ colors: { "lightSlate-50": "255 255 255" } })
    )
    expect(root().style.getPropertyValue("--gdl-text-lightness")).toBe("1")
  })

  it("sets font and radius tokens and resets them on switch", async () => {
    const { applyResolvedTheme } = await load()
    await applyResolvedTheme(
      "a",
      mk({
        tokens: { font: '"X", sans-serif', fontMono: null, radiusScale: 0 }
      }),
      base
    )
    expect(root().style.getPropertyValue("--font")).toBe('"X", sans-serif')
    expect(root().style.getPropertyValue("--radius-scale")).toBe("0")
    await applyResolvedTheme("b", mk(), base)
    expect(root().style.getPropertyValue("--font")).toBe("Ubuntu, sans-serif")
    expect(root().style.getPropertyValue("--font-mono")).toBe("Mono")
    expect(root().style.getPropertyValue("--radius-scale")).toBe("1")
  })

  it("puts the theme css after the app styles and replaces it on switch", async () => {
    const { applyResolvedTheme, THEME_STYLE_ID } = await load()
    await applyResolvedTheme("a", mk({ css: "a{}" }), base)
    const els = document.querySelectorAll(`#${THEME_STYLE_ID}`)
    expect(els).toHaveLength(1)
    const el = els[0]
    expect(document.body.contains(el)).toBe(true)
    expect(el.textContent).toBe("a{}")
    expect(
      document.head.compareDocumentPosition(el) &
        Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy()
    await applyResolvedTheme("b", mk({ css: null }), base)
    const after = document.querySelectorAll(`#${THEME_STYLE_ID}`)
    expect(after).toHaveLength(1)
    expect(after[0]).toBe(el)
    expect(el.textContent).toBe("")
  })

  it("keeps css containing a closing style tag inert", async () => {
    const { applyResolvedTheme, THEME_STYLE_ID } = await load()
    const css = "a{}</style><img src=x onerror=alert(1)>"
    await applyResolvedTheme("a", mk({ css }), base)
    const el = document.getElementById(THEME_STYLE_ID)!
    expect(el.textContent).toBe(css)
    expect(el.children).toHaveLength(0)
    expect(document.querySelector("img")).toBeNull()
  })

  it("adds the theme fonts and removes them when switching", async () => {
    const { applyResolvedTheme } = await load()
    const A = mk({ fonts: [font("FA", "http://x/a.woff2")] })
    await applyResolvedTheme("a", A, base)
    expect(fonts.size).toBe(1)
    await applyResolvedTheme("b", mk(), base)
    expect(fonts.size).toBe(0)
    await applyResolvedTheme("a", A, base)
    expect(fonts.size).toBe(1)
    expect(FakeFontFace.instances).toHaveLength(1)
    expect(FakeFontFace.instances[0].loads).toBe(1)
  })

  it("drops unwanted faces of installed themes but keeps built-in ones", async () => {
    const { applyResolvedTheme } = await load()
    const user = (v: number) =>
      mk({
        fonts: [font("U", `http://x/theme/asset?id=u&path=f.woff2&v=${v}`)]
      })
    await applyResolvedTheme("user:u", user(1), base)
    await applyResolvedTheme("user:u", user(2), base)
    // the first version's face left the document and the cache
    expect(fonts.size).toBe(1)
    expect(FakeFontFace.instances).toHaveLength(2)
    await applyResolvedTheme("user:u", user(1), base)
    expect(FakeFontFace.instances).toHaveLength(3)
    expect(fonts.size).toBe(1)
    // and the same version applied twice is built once
    await applyResolvedTheme("user:u", user(1), base)
    expect(FakeFontFace.instances).toHaveLength(3)

    const builtin = mk({ fonts: [font("B", "/assets/b.woff2")] })
    await applyResolvedTheme("b", builtin, base)
    await applyResolvedTheme("main", mk(), base)
    await applyResolvedTheme("b", builtin, base)
    expect(FakeFontFace.instances).toHaveLength(4)
  })

  it("the last applied theme wins after fast switching", async () => {
    const { applyResolvedTheme, THEME_STYLE_ID } = await load()
    let resolveA!: () => void
    const A = mk({
      css: "a{}",
      fonts: [font("FA", "http://x/a.woff2")]
    })
    const B = mk({ css: "b{}" })
    vi.spyOn(FakeFontFace.prototype, "load").mockImplementation(function (
      this: FakeFontFace
    ) {
      this.loads++
      return new Promise<void>((r) => (resolveA = r))
    })
    const pa = applyResolvedTheme("a", A, base)
    const pb = applyResolvedTheme("b", B, base)
    resolveA()
    await Promise.all([pa, pb])
    expect(root().dataset.gdlTheme).toBe("b")
    expect(document.getElementById(THEME_STYLE_ID)!.textContent).toBe("b{}")
    expect(fonts.size).toBe(0)
  })

  it("marks the root with the theme key", async () => {
    const { applyResolvedTheme } = await load()
    await applyResolvedTheme("user:x", mk(), base)
    expect(root().dataset.gdlTheme).toBe("user:x")
  })

  it("warns for each warning and survives a failed font load", async () => {
    const { applyResolvedTheme } = await load()
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    vi.spyOn(FakeFontFace.prototype, "load").mockRejectedValue(new Error("no"))
    await applyResolvedTheme(
      "a",
      mk({ warnings: ["w1", "w2"], fonts: [font("FA", "http://x/f.woff2")] }),
      base
    )
    expect(warn.mock.calls.length).toBeGreaterThanOrEqual(3)
  })

  it("escapes quotes, backslashes and newlines in a font url", async () => {
    const { applyResolvedTheme } = await load()
    await applyResolvedTheme(
      "a",
      mk({ fonts: [font("FA", 'http://x/a"b\\c\nd.woff2')] }),
      base
    )
    expect(FakeFontFace.instances[0].family).toBe("FA")
    expect(FakeFontFace.instances[0].source).toBe(
      'url("http://x/a\\22 b\\5c c\\a d.woff2")'
    )
  })

  it("publishes the applied theme", async () => {
    const { applyResolvedTheme } = await load()
    const { activeTheme } = await import("../active")
    expect(activeTheme()).toBeNull()
    const a = mk({ id: "A" })
    await applyResolvedTheme("a", a, base)
    expect(activeTheme()).toBe(a)
  })

  it("a hover preview shows the theme but keeps the committed one", async () => {
    const { applyResolvedTheme, THEME_STYLE_ID } = await load()
    const { activeTheme, committedTheme } = await import("../active")
    expect(committedTheme()).toBeNull()
    const saved = mk({ id: "S", css: "s{}" })
    const hovered = mk({ id: "H", css: "h{}" })
    await applyResolvedTheme("user:s", saved, base)
    expect(committedTheme()).toBe(saved)

    await applyResolvedTheme("user:h", hovered, base, { preview: true })
    // the page shows the hovered theme
    expect(activeTheme()).toBe(hovered)
    expect(root().dataset.gdlTheme).toBe("user:h")
    expect(document.getElementById(THEME_STYLE_ID)!.textContent).toBe("h{}")
    // what follows the saved theme doesn't move
    expect(committedTheme()).toBe(saved)

    // the leave puts the saved theme back
    await applyResolvedTheme("user:s", saved, base)
    expect(activeTheme()).toBe(saved)
    expect(committedTheme()).toBe(saved)

    // a preview never commits, even the first apply
    vi.resetModules()
    const fresh = await import("../apply")
    const freshActive = await import("../active")
    await fresh.applyResolvedTheme("user:h", hovered, base, { preview: true })
    expect(freshActive.activeTheme()).toBe(hovered)
    expect(freshActive.committedTheme()).toBeNull()
  })

  it("flags a background on the root", async () => {
    const { applyResolvedTheme } = await load()
    const bg = {
      src: "/s.svg",
      video: false,
      motion: [],
      fit: null,
      position: null,
      dim: null,
      blur: null
    }
    await applyResolvedTheme("a", mk({ background: bg }), base)
    expect(root().hasAttribute("data-gdl-background")).toBe(true)
    await applyResolvedTheme("main", mk(), base)
    expect(root().hasAttribute("data-gdl-background")).toBe(false)
  })

  const railLayout = {
    nav: "rail",
    panels: false,
    tileStyle: "caption",
    tileSize: null
  }
  const flags = () => [
    root().dataset.gdlNav,
    root().dataset.gdlPanels,
    root().dataset.gdlTiles
  ]

  it("flags the layout on the root", async () => {
    const { applyResolvedTheme } = await load()
    await applyResolvedTheme("user:r", mk({ layout: railLayout }), base)
    expect(flags()).toEqual(["rail", "off", "caption"])
    await applyResolvedTheme("main", mk(), base)
    expect(flags()).toEqual(["standard", "on", "overlay"])
  })

  it("a hover preview keeps the saved theme's layout flags", async () => {
    const { applyResolvedTheme } = await load()
    const rail = mk({ layout: railLayout, css: "r{}" })
    await applyResolvedTheme("user:r", rail, base)
    // hovering main while the rail theme is saved
    await applyResolvedTheme("main", mk(), base, { preview: true })
    expect(root().dataset.gdlTheme).toBe("main")
    expect(flags()).toEqual(["rail", "off", "caption"])
    // the leave puts the saved theme back
    await applyResolvedTheme("user:r", rail, base)
    expect(flags()).toEqual(["rail", "off", "caption"])

    // and the other way round: hovering the rail theme while main is saved
    await applyResolvedTheme("main", mk(), base)
    await applyResolvedTheme("user:r", rail, base, { preview: true })
    expect(root().dataset.gdlTheme).toBe("user:r")
    expect(flags()).toEqual(["standard", "on", "overlay"])
  })
})
