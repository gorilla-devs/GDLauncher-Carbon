import { readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { describe, it, expect } from "vitest"
import { GDL_HOOKS, GDL_IDENTITIES } from "../hooks"
import { DECORATION_ANCHORS } from "../decorations"
import {
  SOUND_EVENTS,
  SOUND_GAP_MS,
  SOUND_MAX_MS,
  SOUND_VOLUME
} from "../sounds"
import { PALETTE_TOKENS } from "../palette"
import { PARTICLES_DEFAULTS } from "../effects/particles"

// The authors' guide (packages/ui/themes/README.md) is checked against the
// code it documents: the hook list, the anchors, the sound events and the
// manifest keys the core accepts. A new hook, anchor or key that the guide
// doesn't mention fails here, and so does a guide row for one that is gone.

const root = (path: string) =>
  fileURLToPath(new URL(`../../../../../../../${path}`, import.meta.url))
const read = (path: string) => readFileSync(root(path), "utf8")

const guide = read("packages/ui/themes/README.md")
const extras = read("crates/carbon_app/src/managers/themes/extras.rs")
const manifestRs = read("crates/carbon_app/src/managers/themes/manifest.rs")
const filesRs = read("crates/carbon_app/src/managers/themes/files.rs")

const backticked = new Set([...guide.matchAll(/`([^`\n]+)`/g)].map((m) => m[1]))

/** The text under a heading, up to the next heading of the same or higher level. */
function section(heading: string): string {
  const lines = guide.split("\n")
  const start = lines.findIndex((l) => l.trim() === heading)
  if (start === -1) throw new Error(`the guide has no "${heading}" heading`)
  const level = /^#+/.exec(heading)![0].length
  const out: string[] = []
  for (const line of lines.slice(start + 1)) {
    const m = /^(#+)\s/.exec(line)
    if (m && m[1].length <= level) break
    out.push(line)
  }
  return out.join("\n")
}

/** The first column of every table row under a heading, in order: `| `key` | ...`. */
function keysUnder(heading: string): string[] {
  return [...section(heading).matchAll(/^\|\s*`([^`]+)`\s*\|/gm)].map(
    (m) => m[1]
  )
}

const camel = (s: string) => s.replace(/_([a-z])/g, (_, c) => c.toUpperCase())
const strings = (list: string) =>
  [...list.matchAll(/"([^"]+)"/g)].map((m) => m[1])

/** The key lists the core passes to `warn_unknown`, by the noun it names them with. */
function knownKeys(): Record<string, string[]> {
  const out: Record<string, string[]> = {}
  const re =
    /warn_unknown\(\s*o,\s*(&SOUND_KEYS|&\[[^\]]*\]),\s*[^,]+,\s*"([^"]+)"/g
  for (const m of extras.matchAll(re)) {
    let list = m[1]
    if (list === "&SOUND_KEYS") {
      list = /const SOUND_KEYS: \[&str; \d+\] = \[([^\]]*)\]/.exec(extras)![1]
    }
    out[m[2]] = strings(list)
  }
  return out
}

/** The fields of a serde struct in manifest.rs, as the JSON names. */
function structKeys(name: string): string[] {
  const body = new RegExp(`struct ${name} \\{([\\s\\S]*?)\\n\\}`).exec(
    manifestRs
  )![1]
  const keys: string[] = []
  let rename: string | null = null
  for (const line of body.split("\n")) {
    const r = /#\[serde\(rename = "([^"]+)"\)\]/.exec(line)
    if (r) rename = r[1]
    const f = /^\s+(\w+): /.exec(line)
    if (!f) continue
    if (f[1] !== "extra") keys.push(rename ?? camel(f[1]))
    rename = null
  }
  return keys
}

const sorted = (l: readonly string[]) => [...l].sort()

describe("the theme authors' guide", () => {
  it("lists every hook, in order, and no other", () => {
    for (const hook of GDL_HOOKS) expect(backticked, hook).toContain(hook)
    expect(keysUnder("## Hooks")).toEqual([...GDL_HOOKS])
  })

  it("lists every identity token, by base, and no other", () => {
    const listed: Record<string, string[]> = {}
    for (const base of Object.keys(GDL_IDENTITIES)) {
      const block = /```\n([\s\S]*?)```/.exec(section(`### \`${base}.<id>\``))
      expect(block, base).not.toBeNull()
      listed[base] = block![1].split(/[\s,]+/).filter(Boolean)
    }
    expect(listed).toEqual(
      Object.fromEntries(
        Object.entries(GDL_IDENTITIES).map(([b, ids]) => [b, [...ids]])
      )
    )
    expect(guide).toContain("`<base>.<id>`")
  })

  it("documents the variables a theme sets, and their defaults", () => {
    for (const name of [
      "--gdl-on-primary",
      "--gdl-on-primary-disabled",
      "--gdl-tile-radius",
      "--gdl-group-bleed",
      "--gdl-result-index"
    ]) {
      expect(keysUnder("### Variables"), name).toContain(name)
    }
  })

  it("lists every anchor, sound event and effect", () => {
    for (const anchor of DECORATION_ANCHORS)
      expect(backticked, anchor).toContain(anchor)
    for (const event of SOUND_EVENTS) expect(backticked, event).toContain(event)
    for (const effect of ["snow", "stringLights", "particles"])
      expect(backticked, effect).toContain(effect)
    expect(keysUnder("### Anchors")).toEqual([...DECORATION_ANCHORS])
    expect(sorted(keysUnder("### `sounds`"))).toEqual(sorted(SOUND_EVENTS))
  })

  it("keeps the anchors of the app and the core the same", () => {
    const rust = strings(
      /DECORATION_ANCHORS: \[&str; \d+\] = \[([^\]]*)\]/.exec(extras)![1]
    )
    expect(rust).toEqual([...DECORATION_ANCHORS])
  })

  it("documents every manifest key", () => {
    for (const key of [
      "themeApi",
      "id",
      "name",
      "author",
      "version",
      "description",
      "names",
      "descriptions",
      "preview",
      "colors",
      "fonts",
      "tokens",
      "background",
      "motion",
      "layout",
      "decorations",
      "effects",
      "sounds",
      "css",
      "nav",
      "panels",
      "tileStyle",
      "tileSize"
    ])
      expect(backticked, key).toContain(key)
  })

  it("has a row for every key the core knows, and none it doesn't", () => {
    const known = knownKeys()
    const tables: [string, string[]][] = [
      ["### The top level", structKeys("RawManifest")],
      ["### `fonts`", structKeys("RawFont")],
      ["### `tokens`", structKeys("RawTokens")],
      ["### `background`", known.background],
      ["### `layout`", known.layout],
      ["### `sounds`", known.sound],
      ["#### Decoration entries", known.decoration],
      ["#### `snow`", known.snow],
      ["#### `stringLights`", known.stringLights],
      ["#### `particles`", known.particles]
    ]
    for (const [heading, keys] of tables) {
      expect(keys, heading).toBeDefined()
      expect(sorted(keysUnder(heading)), heading).toEqual(
        sorted(keys.filter((k) => k !== "type"))
      )
    }
  })

  it("lists exactly the palette tokens, as the core knows them", () => {
    const block = /accent, primary\n[\s\S]*?```/.exec(guide)![0]
    const listed = block
      .replace(/```/, "")
      .split(/[\s,]+/)
      .filter(Boolean)
    const rust = /PALETTE_TOKENS: \[&str; \d+\] = \[([^\]]*)\]/.exec(manifestRs)
    expect(PALETTE_TOKENS).toHaveLength(86)
    expect(sorted(listed)).toEqual(sorted(strings(rust![1])))
    expect(sorted(listed)).toEqual(sorted(PALETTE_TOKENS))
  })

  it("states the file limits the core enforces", () => {
    const limit = (name: string) =>
      Number(
        new RegExp(`${name}: ([0-9_]+)`).exec(filesRs)![1].replace(/_/g, "")
      )
    expect(limit("max_total_bytes")).toBe(50 * 1024 * 1024)
    expect(limit("max_files")).toBe(2000)
    expect(limit("max_css_bytes")).toBe(1024 * 1024)
    expect(limit("max_manifest_bytes")).toBe(256 * 1024)
    expect(limit("max_text_bytes")).toBe(256 * 1024)
    for (const text of ["50 MB", "2,000 files", "1 MB", "256 KB"])
      expect(guide, text).toContain(text)
    expect(guide).toContain("| Text            | `txt`, at most 256 KB each.")
    expect(SOUND_MAX_MS).toBe(4000)
    expect(guide).toContain("4 seconds")
  })

  it("prints the particle defaults the renderer uses", () => {
    const rows = section("#### `particles`").matchAll(
      /^\|\s*`(\w+)`\s*\|\s*([^|]*?)\s*\|/gm
    )
    const printed: Record<string, number | boolean> = {}
    for (const [, key, value] of rows) {
      if (!value) continue
      printed[key] =
        value === "`true`" || value === "`false`"
          ? value === "`true`"
          : Number(value)
    }
    expect(printed).toEqual({ ...PARTICLES_DEFAULTS })
  })

  it("prints the sound volume and each event's gap", () => {
    const volume = /fixed and low \((\d+)%\)/.exec(guide)
    expect(volume, "the volume line").not.toBeNull()
    expect(Number(volume![1]) / 100).toBe(SOUND_VOLUME)

    const line = guide
      .split("\n")
      .find((l) => l.includes("can't repeat too fast"))
    expect(line, "the gap line").toBeDefined()
    const gaps: Record<string, number> = {}
    // "`launch` at most every 2 s", "`notification` and `error` every 0.5 s"
    for (const [, events, seconds] of line!.matchAll(
      /((?:`\w+`(?:, | and )?)+)\s+(?:at most )?every ([\d.]+) s\b/g
    )) {
      for (const [, event] of events.matchAll(/`(\w+)`/g)) {
        gaps[event] = Math.round(Number(seconds) * 1000)
      }
    }
    expect(gaps).toEqual(SOUND_GAP_MS)
  })

  it("names the reset shortcut and the glass rule", () => {
    for (const text of [
      "Ctrl+Alt+Shift+T",
      "Cmd+Option+Shift+T",
      "--gdl-panel-inset",
      "--gdl-panel-radius",
      "backdrop-filter"
    ])
      expect(guide, text).toContain(text)
  })
})
