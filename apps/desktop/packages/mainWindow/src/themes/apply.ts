import type { FEResolvedTheme } from "@gd/core_module/bindings"
import { PALETTE_TOKENS } from "./palette"
import { setActiveTheme } from "./active"
import { resolveLayout } from "./layout"

export const THEME_STYLE_ID = "gdl-theme-css"

interface Entry {
  face: FontFace
  started: boolean
  /** Served from an installed theme's folder, so its url changes on every edit. */
  user: boolean
}

// reused across applies, so a face is built and loaded once. Faces of installed
// themes are dropped as soon as they are no longer wanted (a live edit changes
// the `v=` in their url); built-in ones are few and stay cached.
const faces = new Map<string, Entry>()
let active = new Set<string>()

export function cssString(value: string) {
  return `"${value.replace(/[\\"\n\r]/g, (c) => `\\${c.charCodeAt(0).toString(16)} `)}"`
}

/** The average of an `"r g b"` palette value's channels, 0 to 1, rounded to
 *  three places; null when it isn't one. */
export function textLightness(rgb: string | null | undefined): number | null {
  const parts = (rgb ?? "").trim().split(/\s+/).map(Number)
  if (parts.length !== 3 || parts.some((n) => !Number.isFinite(n))) return null
  return Math.round(((parts[0] + parts[1] + parts[2]) / 765) * 1000) / 1000
}

export interface ApplyOptions {
  /** A hover preview: the page shows the theme, committedTheme() keeps the saved one. */
  preview?: boolean
}

/**
 * Puts a resolved theme on the page. Every DOM change happens before the
 * first await, so the last call always wins; the promise only waits for the
 * fonts to load. `base` fills palette tokens and fonts the theme leaves out.
 */
export function applyResolvedTheme(
  key: string,
  theme: FEResolvedTheme,
  base: FEResolvedTheme,
  opts: ApplyOptions = {}
): Promise<void> {
  const rootEl = document.documentElement
  const style = rootEl.style

  for (const token of PALETTE_TOKENS) {
    const value = theme.colors[token] ?? base.colors[token]
    if (value === undefined) style.removeProperty(`--${token}`)
    else style.setProperty(`--${token}`, value)
  }
  // How light the main text colour is, 0 to 1. CSS can't read one number out
  // of a palette token, and parts on a fixed dark backing switch to light
  // words from it (themes/controls.css).
  const text = textLightness(
    theme.colors["lightSlate-50"] ?? base.colors["lightSlate-50"]
  )
  if (text === null) style.removeProperty("--gdl-text-lightness")
  else style.setProperty("--gdl-text-lightness", String(text))

  const setOrRemove = (name: string, value: string | null | undefined) => {
    if (value == null) style.removeProperty(name)
    else style.setProperty(name, value)
  }
  setOrRemove("--font", theme.tokens.font ?? base.tokens.font)
  setOrRemove("--font-mono", theme.tokens.fontMono ?? base.tokens.fontMono)
  style.setProperty("--radius-scale", String(theme.tokens.radiusScale ?? 1))

  const loads: Promise<unknown>[] = []
  if (typeof FontFace !== "undefined" && document.fonts) {
    const wanted = new Set<string>()
    for (const f of theme.fonts) {
      const id = `${f.family}|${f.url}|${f.weight}|${f.style}`
      wanted.add(id)
      let entry = faces.get(id)
      if (!entry) {
        const descriptors: FontFaceDescriptors = {}
        if (f.weight) descriptors.weight = f.weight
        if (f.style) descriptors.style = f.style
        entry = {
          face: new FontFace(f.family, `url(${cssString(f.url)})`, descriptors),
          started: false,
          user: f.url.includes("/theme/asset?")
        }
        faces.set(id, entry)
      }
      document.fonts.add(entry.face)
      if (!entry.started) {
        entry.started = true
        loads.push(entry.face.load())
      }
    }
    for (const id of new Set([...active, ...faces.keys()])) {
      if (wanted.has(id)) continue
      const entry = faces.get(id)
      if (!entry) continue
      document.fonts.delete(entry.face)
      if (entry.user) faces.delete(id)
    }
    active = wanted
  }

  let el = document.getElementById(THEME_STYLE_ID)
  if (!el) {
    el = document.createElement("style")
    el.id = THEME_STYLE_ID
    // body content comes after every head style and link
    document.body.appendChild(el)
  }
  el.textContent = theme.css ?? ""

  rootEl.dataset.gdlTheme = key
  rootEl.toggleAttribute("data-gdl-background", theme.background !== null)
  // A hover preview keeps the saved theme's layout (as layout() does): a nav
  // or tiles that changed under the cursor would move the tile being hovered.
  if (opts.preview !== true) {
    const layout = resolveLayout(theme)
    rootEl.dataset.gdlNav = layout.nav
    rootEl.dataset.gdlPanels = layout.panels ? "on" : "off"
    rootEl.dataset.gdlTiles = layout.tileStyle
  }
  setActiveTheme(theme, { preview: opts.preview === true })

  for (const w of theme.warnings) console.warn(w)

  return Promise.allSettled(loads).then((results) => {
    for (const r of results) {
      if (r.status === "rejected")
        console.warn("Theme font failed to load", r.reason)
    }
  })
}
