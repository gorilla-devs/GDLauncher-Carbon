import { createMemo, createRoot } from "solid-js"
import type { FEResolvedTheme } from "@gd/core_module/bindings"
import { committedTheme } from "./active"

export type NavStyle = "standard" | "compact" | "rail"

export interface ResolvedLayout {
  nav: NavStyle
  panels: boolean
  tileStyle: "overlay" | "caption"
  tileSize: number | null
}

export const LAYOUT_DEFAULTS: ResolvedLayout = Object.freeze({
  nav: "standard",
  panels: true,
  tileStyle: "overlay",
  tileSize: null
})

const NAVS: readonly NavStyle[] = ["standard", "compact", "rail"]
const TILE_STYLES: readonly ResolvedLayout["tileStyle"][] = [
  "overlay",
  "caption"
]

/**
 * The layout a theme asks for, with the defaults filled in. The core checks
 * these values too, but the bindings type them as plain strings and numbers
 * and built-ins never pass through the core, so anything unexpected falls back
 * to the default here as well.
 */
export function resolveLayout(
  theme: Pick<FEResolvedTheme, "layout"> | null | undefined
): ResolvedLayout {
  const l = theme?.layout
  const size = l?.tileSize
  return {
    nav: NAVS.find((n) => n === l?.nav) ?? LAYOUT_DEFAULTS.nav,
    panels: l?.panels !== false,
    tileStyle:
      TILE_STYLES.find((s) => s === l?.tileStyle) ?? LAYOUT_DEFAULTS.tileStyle,
    tileSize:
      typeof size === "number" &&
      Number.isInteger(size) &&
      size >= 1 &&
      size <= 5
        ? size
        : null
  }
}

export function sameLayout(a: ResolvedLayout, b: ResolvedLayout): boolean {
  return (
    a.nav === b.nav &&
    a.panels === b.panels &&
    a.tileStyle === b.tileStyle &&
    a.tileSize === b.tileSize
  )
}

// Follows the saved theme, never a tile's hover preview: a nav or tiles that
// changed under the cursor would move the tile being hovered. Compared by
// value, because every hover's leave commits a new theme object.
const committedLayout = createRoot(() =>
  createMemo(() => resolveLayout(committedTheme()), undefined, {
    equals: sameLayout
  })
)

/** The saved theme's layout; reactive. */
export function layout(): ResolvedLayout {
  return committedLayout()
}

/**
 * The library's tile size: the one the user picked in the filter menu, or else
 * the theme's default, or else the stored size.
 */
export function effectiveTileSize(
  stored: number | undefined,
  chosen: boolean | undefined,
  themeSize: number | null
): number {
  if (chosen === true) return stored ?? 2
  return themeSize ?? stored ?? 2
}

export const NAV_METRICS = {
  standard: { height: 60, logo: 36, search: 40 },
  compact: { height: 48, logo: 28, search: 32 },
  rail: { width: 64 }
} as const

/** Where main puts the ad view's top (its NAVBAR_HEIGHT; a test pins them). */
export const AD_TOP = 60

/** The width the nav takes at the window's left edge. */
export function railWidth(nav: NavStyle): number {
  return nav === "rail" ? NAV_METRICS.rail.width : 0
}

/**
 * Space above the ad column's placeholders, so they start where main puts the
 * ad view (which never moves): AD_TOP less the height of the bar above the
 * column. The rail has no bar there.
 */
export function adColumnSpacer(nav: NavStyle): number {
  return AD_TOP - (nav === "rail" ? 0 : NAV_METRICS[nav].height)
}

/**
 * The rail has no room for the search box, so the pages that keep it expanded
 * (search and an addon's page, the box's own `/search/*` and `/addon/*`) get
 * it in a strip above the page.
 */
export function showsSearchStrip(nav: NavStyle, pathname: string): boolean {
  if (nav !== "rail") return false
  return (
    pathname === "/search" ||
    pathname.startsWith("/search/") ||
    pathname.startsWith("/addon/")
  )
}
