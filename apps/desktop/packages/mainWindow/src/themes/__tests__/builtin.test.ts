import { describe, it, expect } from "vitest"
import {
  BUILTIN_THEME_IDS,
  type BuiltinThemeId,
  type Manifest,
  getBuiltinSeasonArt,
  getBuiltinSeasonWindow,
  getBuiltinTheme,
  resolveBuiltinAsset,
  resolveBuiltinManifest
} from "../builtin"
import { PALETTE_TOKENS } from "../palette"

const manifests = import.meta.glob(
  "../../../../../../../packages/ui/themes/*/manifest.json",
  { eager: true, import: "default" }
)

describe("built-in themes", () => {
  it("lists one built-in per package folder", () => {
    const folders = Object.keys(manifests).map(
      (k) => k.split("/themes/")[1].split("/")[0]
    )
    expect(folders.length).toBe(7)
    expect([...BUILTIN_THEME_IDS].sort()).toEqual(folders.sort())
  })

  it("every built-in sets every palette token", () => {
    for (const id of BUILTIN_THEME_IDS) {
      expect(Object.keys(getBuiltinTheme(id).colors).sort()).toEqual(
        [...PALETTE_TOKENS].sort()
      )
    }
  })

  it("resolves win95's font to a bundled URL", () => {
    expect(getBuiltinTheme("win95").fonts).toEqual([
      {
        family: "W95FA",
        weight: "400",
        style: "normal",
        url: expect.stringContaining("w95fa")
      }
    ])
  })

  const manifest = (background: Manifest["background"]): Manifest => ({
    id: "x",
    name: "X",
    author: "a",
    version: "1",
    background,
    decorations: [
      {
        anchor: "nav",
        edge: "top",
        images: ["assets/a.svg", "assets/b.svg"]
      }
    ],
    effects: [{ type: "snow" as const, density: 10 }]
  })
  const resolve = (p: string) => "/x/" + p

  it("maps backgrounds, decorations and effects through the resolver", () => {
    const t = resolveBuiltinManifest(
      manifest({ src: "assets/s.svg", motion: ["assets/m.svg"] }),
      resolve,
      null
    )
    expect(t.background?.src).toBe("/x/assets/s.svg")
    expect(t.background?.motion).toEqual(["/x/assets/m.svg"])
    expect(t.background?.video).toBe(false)
    expect(t.decorations[0].images).toEqual([
      "/x/assets/a.svg",
      "/x/assets/b.svg"
    ])
    expect(t.effects).toEqual([
      {
        type: "snow",
        density: 10,
        speed: null,
        minSize: null,
        maxSize: null,
        wind: null,
        color: null,
        layers: null
      }
    ])
  })

  it("marks video backgrounds", () => {
    const t = resolveBuiltinManifest(
      manifest({ src: "assets/v.webm" }),
      resolve,
      null
    )
    expect(t.background?.video).toBe(true)
  })

  it("themes without extras get empty lists", () => {
    const t = getBuiltinTheme("main")
    expect(t.background).toBeNull()
    expect(t.decorations).toEqual([])
    expect(t.effects).toEqual([])
  })

  it("silent night resolves its sounds", () => {
    const { sounds } = getBuiltinTheme("silent-night")
    expect(sounds.launch).toContain("sleigh-bells")
    expect(sounds.downloadComplete).toContain("chime")
    expect(sounds.click).toBeNull()
    expect(sounds.notification).toBeNull()
    expect(sounds.error).toBeNull()
  })

  it("maps layout, sounds and particles through the resolver", () => {
    const m = manifest(undefined)
    m.layout = { nav: "rail", panels: false, tileSize: 4 }
    m.sounds = {
      launch: "assets/sounds/l.wav",
      downloadComplete: "assets/sounds/d.wav"
    }
    m.effects = [
      {
        type: "particles",
        images: ["assets/p1.png", "assets/p2.png"],
        preset: "drift",
        minSize: 10
      }
    ]
    const t = resolveBuiltinManifest(m, resolve, null)
    expect(t.layout).toEqual({
      nav: "rail",
      panels: false,
      tileStyle: null,
      tileSize: 4
    })
    expect(t.sounds).toEqual({
      launch: "/x/assets/sounds/l.wav",
      downloadComplete: "/x/assets/sounds/d.wav",
      click: null,
      notification: null,
      error: null
    })
    expect(t.effects).toEqual([
      {
        type: "particles",
        images: ["/x/assets/p1.png", "/x/assets/p2.png"],
        preset: "drift",
        density: null,
        speed: null,
        minSize: 10,
        maxSize: null,
        rotate: null,
        opacity: null
      }
    ])
  })

  it("themes without them get empty values", () => {
    const t = getBuiltinTheme("main")
    expect(t.layout).toEqual({
      nav: null,
      panels: null,
      tileStyle: null,
      tileSize: null
    })
    expect(t.sounds).toEqual({
      launch: null,
      downloadComplete: null,
      click: null,
      notification: null,
      error: null
    })
  })

  it("normalizes string lights", () => {
    const m = manifest(undefined)
    m.effects = [{ type: "stringLights", twinkle: 0.5 }]
    expect(resolveBuiltinManifest(m, resolve, null).effects).toEqual([
      {
        type: "stringLights",
        colors: [],
        spacing: null,
        droop: null,
        size: null,
        twinkle: 0.5,
        bulb: null
      }
    ])
  })

  it("ignores the season in the resolved theme", () => {
    const m = manifest(undefined)
    m.season = { available: ["12-01", "01-31"], art: "assets/a.png" }
    expect(resolveBuiltinManifest(m, resolve, null)).not.toHaveProperty(
      "season"
    )
  })

  it("has no season art for a built-in without it", () => {
    expect(getBuiltinSeasonArt("main")).toBeNull()
  })

  it("reads a seasonal window from the manifest", () => {
    expect(getBuiltinSeasonWindow("silent-night")).toEqual({
      from: "12-01",
      until: "01-31"
    })
    expect(getBuiltinSeasonWindow("main")).toBeNull()
  })

  it("silent night resolves its artwork", () => {
    const t = getBuiltinTheme("silent-night")
    expect(t.background?.video).toBe(false)
    expect(t.background?.motion).toHaveLength(1)
    expect(t.decorations).toHaveLength(4)
    const tile = t.decorations.find((d) => d.anchor === "library.tile")
    expect(tile?.images).toHaveLength(16)
    for (const url of tile?.images ?? []) expect(url).toContain("cap-")
    expect(t.effects).toHaveLength(2)
    expect(t.warnings).toEqual([])
    expect(getBuiltinSeasonArt("silent-night")).toContain("dialog-art")
  })

  it("resolves every asset path every built-in manifest references", () => {
    const missing: string[] = []
    for (const [file, m] of Object.entries(manifests) as [string, Manifest][]) {
      const id = file.split("/themes/")[1].split("/")[0] as BuiltinThemeId
      const paths = [
        ...(m.fonts ?? []).map((f) => f.src),
        ...(m.background
          ? [m.background.src, ...(m.background.motion ?? [])]
          : []),
        ...(m.decorations ?? []).flatMap((d) => d.images ?? []),
        ...(m.season?.art ? [m.season.art] : [])
      ]
      for (const p of paths)
        if (!resolveBuiltinAsset(id, p)) missing.push(`${id}: ${p}`)
    }
    expect(missing).toEqual([])
  })
})
