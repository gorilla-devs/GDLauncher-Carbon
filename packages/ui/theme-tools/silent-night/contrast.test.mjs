import { describe, it, expect } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const here = path.dirname(fileURLToPath(import.meta.url))
const manifest = JSON.parse(
  fs.readFileSync(
    path.resolve(here, "../../themes/silent-night/manifest.json"),
    "utf8"
  )
)

const themeDir = path.resolve(here, "../../themes/silent-night")
const css = fs.readFileSync(path.join(themeDir, "theme.css"), "utf8")
const scene = fs.readFileSync(path.join(themeDir, "assets/scene.svg"), "utf8")

// The colours below are asserted to appear in theme.css, so the two can't
// drift apart.
const GOLD_TEXT = "#3a2408"
const TITLE = "#f3deaf"
const GLASS_TOP = [22, 37, 72]
const GLASS_TOP_ALPHA = 0.78
const GLASS_TOP_CSS = `rgb(${GLASS_TOP.join(" ")} / ${GLASS_TOP_ALPHA})`
const GOLD_DARK = "#eaa94b" // the gold gradient's darker end
const PLACEHOLDER = "#8794b5" // the mock's search placeholder
const MOON = [255, 245, 222] // brightest scene point
const MOON_GLOW = "#fff2d2" // the moon glow's centre stop in scene.svg
const NAV_END = [7, 14, 32] // the nav gradient's lighter, see-through end
const NAV_END_ALPHA = 0.62
const NAV_END_CSS = `rgb(${NAV_END.join(" ")} / ${NAV_END_ALPHA})`
const FIELD_ALPHA = 0.05 // the white glass of the inputs and search boxes
const FIELD_CSS = `rgb(255 255 255 / ${FIELD_ALPHA})`
const WINDOW_H = 986
const NAV_H = 60
const PANEL_TOP = 76 // nav 60 + frame padding 16

// The sky gradient of scene.svg, sampled at the panel's centre row.
const skyAt = (offset) => {
  const g = scene.match(
    /<linearGradient id="sky"[^>]*>(.*?)<\/linearGradient>/
  )[1]
  const stops = [
    ...g.matchAll(/offset="([\d.]+)" stop-color="(#[0-9a-f]{6})"/g)
  ].map((m) => [Number(m[1]), hex(m[2])])
  for (let i = 1; i < stops.length; i++) {
    if (offset <= stops[i][0]) {
      const [o0, c0] = stops[i - 1]
      const [o1, c1] = stops[i]
      return mix(c1, (offset - o0) / (o1 - o0), c0)
    }
  }
  return stops[stops.length - 1][1]
}

const rgb = (token) => manifest.colors[token].split(" ").map(Number)
const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16))

// WCAG 2.x relative luminance and contrast ratio.
const channel = (v) => {
  const c = v / 255
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}
const luminance = ([r, g, b]) =>
  0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
const ratio = (a, b) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}
const mix = (top, alpha, under) =>
  top.map((v, i) => alpha * v + (1 - alpha) * under[i])

describe("silent night contrast", () => {
  it("has the full palette", () => {
    expect(Object.keys(manifest.colors)).toHaveLength(86)
  })

  it("keeps the tested colours in theme.css", () => {
    expect(css.toLowerCase()).toContain(GOLD_TEXT)
    expect(css.toLowerCase()).toContain(TITLE)
    expect(css.toLowerCase()).toContain(GOLD_DARK)
    expect(css.toLowerCase()).toContain(PLACEHOLDER)
    expect(css).toContain(GLASS_TOP_CSS)
    expect(css).toContain(NAV_END_CSS)
    expect(css).toContain(FIELD_CSS)
  })

  // The nav search box is glass too: its white tint over the nav, over the
  // sky at the top of the window, with the moon's glow at its 0.14 stop
  // (more than reaches the nav at the mock's size) lighting the sky.
  describe("on the nav search box", () => {
    const lit = mix(hex(MOON_GLOW), 0.14, skyAt(NAV_H / 2 / WINDOW_H))
    const box = mix(
      [255, 255, 255],
      FIELD_ALPHA,
      mix(NAV_END, NAV_END_ALPHA, lit)
    )

    it("styles the box with the field glass", () => {
      const rule = css
        .match(/([^{}]*)\{[^}]*\}/g)
        .find((r) => r.includes('[data-gdl~="nav.search.box"]'))
      expect(rule, "a rule for nav.search.box").toBeDefined()
      expect(rule).toContain(FIELD_CSS)
      expect(scene.toLowerCase()).toContain(MOON_GLOW)
    })

    it("a placeholder is at least 4.5", () => {
      expect(ratio(hex(PLACEHOLDER), box)).toBeGreaterThanOrEqual(4.5)
    })

    it("the typed text (lightSlate-50) is at least 7", () => {
      expect(ratio(rgb("lightSlate-50"), box)).toBeGreaterThanOrEqual(7)
    })

    // The icon is dim in every theme; the glass must not make it dimmer.
    it("the search icon (darkSlate-400) keeps the contrast it had on the opaque box", () => {
      expect(ratio(rgb("darkSlate-400"), box)).toBeGreaterThanOrEqual(
        0.9 * ratio(rgb("darkSlate-400"), rgb("darkSlate-700"))
      )
    })
  })

  it("a placeholder on darkSlate-700 (an opaque field) is at least 4.5", () => {
    expect(
      ratio(hex(PLACEHOLDER), rgb("darkSlate-700"))
    ).toBeGreaterThanOrEqual(4.5)
  })

  it("keeps the moon colour in scene.svg", () => {
    const h = (v) => v.toString(16).padStart(2, "0")
    expect(scene.toLowerCase()).toContain(`#${MOON.map(h).join("")}`)
  })

  it("lightSlate-50 on darkSlate-800 is at least 7", () => {
    expect(
      ratio(rgb("lightSlate-50"), rgb("darkSlate-800"))
    ).toBeGreaterThanOrEqual(7)
  })

  it("lightSlate-700 on darkSlate-800 is at least 4.5", () => {
    expect(
      ratio(rgb("lightSlate-700"), rgb("darkSlate-800"))
    ).toBeGreaterThanOrEqual(4.5)
  })

  it("lightSlate-900 on darkSlate-700 is at least 4.5", () => {
    expect(
      ratio(rgb("lightSlate-900"), rgb("darkSlate-700"))
    ).toBeGreaterThanOrEqual(4.5)
  })

  it("the gold fills' text on primary-500 is at least 4.5", () => {
    expect(ratio(hex(GOLD_TEXT), rgb("primary-500"))).toBeGreaterThanOrEqual(
      4.5
    )
  })

  it("the gold text on the gold gradient's darker end is at least 4.5", () => {
    expect(ratio(hex(GOLD_TEXT), hex(GOLD_DARK))).toBeGreaterThanOrEqual(4.5)
  })

  it("the section title on darkSlate-800 is at least 4.5", () => {
    expect(ratio(hex(TITLE), rgb("darkSlate-800"))).toBeGreaterThanOrEqual(4.5)
  })

  describe("on the fade bands", () => {
    // Each band hook and the glass colour and alpha its rule must use.
    const BANDS = {
      "instance.header.fade": [12, 22, 48, 0.78],
      "server.header.fade": [12, 22, 48, 0.78],
      "search.sidebar.fade": [12, 22, 48, 0.78],
      "search.context.fade": [12, 22, 48, 0.78],
      "search.context.shade": [12, 22, 48, 0.6]
    }
    const ruleOf = (hook) =>
      css
        .match(/([^{}]*)\{[^}]*\}/g)
        .find((r) => r.split("{")[0].includes(`[data-gdl~="${hook}"]`))

    it("restyles each fade hook with its glass colour", () => {
      for (const [hook, [r, g, b, a]] of Object.entries(BANDS)) {
        const rule = ruleOf(hook)
        expect(rule, `a rule for ${hook}`).toBeDefined()
        expect(rule, hook).toContain(`rgb(${r} ${g} ${b} / ${a})`)
      }
    })

    // A band's densest point is its colour at its alpha over the scene; text
    // on it must stay readable there, over the brightest scene point.
    it("lightSlate-50 is at least 4.5 on every band's densest point over the moon", () => {
      for (const [hook, [r, g, b, a]] of Object.entries(BANDS))
        expect(
          ratio(rgb("lightSlate-50"), mix([r, g, b], a, MOON)),
          hook
        ).toBeGreaterThanOrEqual(4.5)
    })
  })

  describe("on the panel glass", () => {
    // The glass's top colour over the brightest scene point (the moon), and
    // over the sky at the panel's centre row.
    const worst = mix(GLASS_TOP, GLASS_TOP_ALPHA, MOON)
    const worstAtBriefAlpha = mix(GLASS_TOP, 0.7, MOON)
    const typical = mix(
      GLASS_TOP,
      GLASS_TOP_ALPHA,
      skyAt((PANEL_TOP + WINDOW_H) / 2 / WINDOW_H)
    )

    it("lightSlate-50 is at least 4.5 over the moon", () => {
      expect(ratio(rgb("lightSlate-50"), worst)).toBeGreaterThanOrEqual(4.5)
    })

    it("the section title is at least 4.5 over the moon, at the brief's alpha too", () => {
      expect(ratio(hex(TITLE), worst)).toBeGreaterThanOrEqual(4.5)
      expect(ratio(hex(TITLE), worstAtBriefAlpha)).toBeGreaterThanOrEqual(4.5)
    })

    it("lightSlate-700 is at least 4.5 over the typical sky", () => {
      expect(ratio(rgb("lightSlate-700"), typical)).toBeGreaterThanOrEqual(4.5)
    })

    it("a placeholder is at least 4.5 over the typical sky", () => {
      expect(ratio(hex(PLACEHOLDER), typical)).toBeGreaterThanOrEqual(4.5)
    })

    it("lightSlate-700 is at least 3 over the moon", () => {
      expect(ratio(rgb("lightSlate-700"), worst)).toBeGreaterThanOrEqual(3)
    })

    it("the glass's top alpha stays within 0.82", () => {
      expect(GLASS_TOP_ALPHA).toBeLessThanOrEqual(0.82)
    })
  })
})
