import type { FEThemeDecoration } from "@gd/core_module/bindings"
import { cssString } from "./apply"

// Decorations are static images drawn by <DecorationSlot/>, which the app
// renders inside each anchor. Plain TS (no Solid) so it can be unit tested.
export const DECORATION_ANCHORS = [
  "content",
  "nav",
  "nav.logo",
  "panel",
  "library.tile",
  "modal",
  "season-dialog",
  "section-title",
  "library.group",
  "library.toolbar",
  "instance.header",
  "server.header",
  "settings.nav",
  "page-title"
] as const
export type DecorationAnchor = (typeof DECORATION_ANCHORS)[number]

// Slots of these anchors clip to their own anchor box. `content` is the
// column that ends where the ad column starts, `nav` spans the whole window
// above the ad column, and `modal` is a rounded panel. Nothing a theme draws
// may reach the ad column. (`season-dialog` is not clipped: its cap hangs over
// the dialog's edge. The dialog's row clips it sideways at the ad column's
// edge instead, MODAL_ROW_CLIP.) `nav.logo` is bounded by the nav band
// instead, see Navbar.tsx.
//
// `panel` is deliberately not clipped by the slot: its slot is a sibling over
// the panel box inside `panel-frame`, which is `overflow-hidden` and has 16 px
// of padding above the panel. So a ledge on the panel's top edge (`y` down to
// -16px) shows exactly that much above the panel, and anything further is cut
// by the frame.
const CLIPPED_ANCHORS: ReadonlySet<string> = new Set([
  "content",
  "nav",
  "modal"
])

/**
 * Inline style for the navs (the top bar, the rail and the rail's search
 * strip): bounds everything in the nav's own box, so a `nav.logo` decoration
 * can't run right/down into the ad column. No clip margin: the ad view starts
 * right under the standard bar, so any room below would let a decoration
 * paint the view's top rows. Not a stacking context.
 */
export const NAV_BAND_CLIP = {
  overflow: "clip"
} as const

/** Inline style for the content column: nothing paints past its right edge. */
export const CONTENT_COLUMN_CLIP = { "overflow-x": "clip" } as const

/**
 * Inline style for the left part of a dialog's row (the dialog and its dim),
 * whose right edge is the ad column's left edge: nothing in a dialog paints
 * past it, not even under the column's 95% dim. Only x (`clip`, unlike
 * `hidden`, leaves y visible), so a season cap still hangs over the top.
 */
export const MODAL_ROW_CLIP = { "overflow-x": "clip" } as const

export function slotClass(anchor: string, extra?: string): string {
  return [
    "pointer-events-none absolute inset-0 z-[1]",
    CLIPPED_ANCHORS.has(anchor) ? "[overflow:clip]" : "",
    extra ?? ""
  ]
    .filter(Boolean)
    .join(" ")
}

/** FNV-1a 32-bit over UTF-16 code units, unsigned. */
export function hashKey(s: string): number {
  let h = 2166136261
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0
}

// FNV-1a's low bits alternate for sequential ids, which would make a
// power-of-two variant count strictly alternate; this spreads the high bits down.
function mix(h: number): number {
  h ^= h >>> 16
  h = Math.imul(h, 0x45d9f3b)
  h ^= h >>> 16
  return h >>> 0
}

const cycleSeen = new Map<string, number>()

/** Tests only. */
export function resetCycleRegistry(): void {
  cycleSeen.clear()
}

export function pickVariant(
  pick: string | null,
  count: number,
  key: string,
  decorationIndex: number,
  anchor: string
): number {
  if (count <= 1) return 0
  if (pick === "random")
    return mix(hashKey(`${decorationIndex}:${key}`)) % count
  if (pick === "cycle") {
    const id = `${anchor}|${key}`
    let n = cycleSeen.get(id)
    if (n === undefined) {
      n = cycleSeen.size
      cycleSeen.set(id, n)
    }
    return n % count
  }
  return 0
}

export function cssLength(
  v: number | string | null | undefined
): string | null {
  if (v === null || v === undefined) return null
  return typeof v === "number" ? `${v}px` : v
}

function negate(v: string): string {
  return v.startsWith("-") ? v.slice(1) : `-${v}`
}

export function decorationStyle(
  d: FEThemeDecoration,
  url: string
): Record<string, string> {
  const x = cssLength(d.x) ?? "0px"
  const y = cssLength(d.y) ?? "0px"
  const w = cssLength(d.width)
  const h = cssLength(d.height)
  const style: Record<string, string> = {
    position: "absolute",
    "background-image": `url(${cssString(url)})`,
    "background-position": "left top"
  }
  const horizontal = d.edge.endsWith("right") ? "right" : "left"
  const vertical = d.edge.startsWith("bottom") ? "bottom" : "top"
  if (horizontal === "left") style.left = x
  else style.right = negate(x)
  if (vertical === "top") style.top = y
  else style.bottom = negate(y)

  if (d.edge === "top" || d.edge === "bottom") {
    style.width = w ?? "100%"
    if (h) style.height = h
  } else if (d.edge === "left" || d.edge === "right") {
    if (w) style.width = w
    style.height = h ?? "100%"
  } else {
    if (w) style.width = w
    if (h) style.height = h
  }

  if (d.repeat === "x") {
    style["background-size"] = "auto 100%"
    style["background-repeat"] = "repeat-x"
  } else if (d.repeat === "y") {
    style["background-size"] = "100% auto"
    style["background-repeat"] = "repeat-y"
  } else {
    style["background-size"] = "100% 100%"
    style["background-repeat"] = "no-repeat"
  }
  return style
}

/** Whether `theme` draws anything in `anchor` for `key`. */
export function hasDecorations(
  theme: { decorations: FEThemeDecoration[] } | null | undefined,
  anchor: string,
  key: string
): boolean {
  return slotDecorations(theme, anchor, key).length > 0
}

/** The decorations of one anchor, with the variant each one shows for `key`. */
export function slotDecorations(
  theme: { decorations: FEThemeDecoration[] } | null | undefined,
  anchor: string,
  key: string
): { index: number; style: Record<string, string> }[] {
  const out: { index: number; style: Record<string, string> }[] = []
  theme?.decorations.forEach((d, index) => {
    if (d.anchor !== anchor || d.images.length === 0) return
    const v = pickVariant(d.pick, d.images.length, key, index, anchor)
    out.push({ index, style: decorationStyle(d, d.images[v]) })
  })
  return out
}
