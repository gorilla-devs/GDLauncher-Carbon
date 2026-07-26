import { describe, expect, it } from "vitest"
import {
  deriveLoaderVersionSeed,
  loaderVersionIdFromTestId,
  pickSeededOption,
  type Loader
} from "./instances.js"

// A realistic-length list, shaped like a real Forge version dropdown for one
// Minecraft release (Task 3's live DOM probe against 26.2 returned exactly
// this many options — see task-3-report.md).
const OPTIONS = [
  "26.2-65.0.9",
  "26.2-65.0.8",
  "26.2-65.0.7",
  "26.2-65.0.6",
  "26.2-65.0.5",
  "26.2-65.0.4",
  "26.2-65.0.3",
  "26.2-65.0.2",
  "26.2-65.0.1",
  "26.2-65.0.0"
]

describe("pickSeededOption", () => {
  it("is deterministic for a given seed", () => {
    const a = pickSeededOption(OPTIONS, 12345)
    const b = pickSeededOption(OPTIONS, 12345)
    expect(a).toBe(b)
  })

  it("lets the seed actually drive the draw", () => {
    // Asserting that two specific seeds differ would be a coin flip, not a
    // test: any given pair can legitimately collide on a 10-element list.
    // The real property is that the seed is consulted at all, so sample many
    // seeds and require the draws to spread across the option list.
    const distinct = new Set(
      Array.from({ length: 30 }, (_, seed) => pickSeededOption(OPTIONS, seed))
    )
    expect(distinct.size).toBeGreaterThan(5)
  })

  it("always returns one of the offered options, never a fabricated value", () => {
    for (let seed = 0; seed < 30; seed++) {
      expect(OPTIONS).toContain(pickSeededOption(OPTIONS, seed))
    }
  })

  it("returns the sole entry for a one-element list", () => {
    for (let seed = 0; seed < 10; seed++) {
      expect(pickSeededOption(["26.2-65.0.9"], seed)).toBe("26.2-65.0.9")
    }
  })

  it("throws a descriptive error on an empty list", () => {
    expect(() => pickSeededOption([], 1)).toThrow(/empty/i)
  })
})

describe("loaderVersionIdFromTestId", () => {
  it("recovers the id from a well-formed data-testid", () => {
    expect(
      loaderVersionIdFromTestId(
        "instance-creation-loader-version-option-1.20.1-47.4.22"
      )
    ).toBe("1.20.1-47.4.22")
  })

  it("recovers the id of whichever option Custom.tsx pre-selects (index 0), unaffected by its checkmark markup", () => {
    // Regression guard: `SelectItem` (packages/ui/src/Select/index.tsx)
    // renders a `<svg><title>Checked</title></svg>` ahead of the label for
    // whichever option is currently selected, and `Custom.tsx` pre-selects
    // the first loader-version option before the dropdown is ever opened.
    // `readLoaderVersionOptions` used to recover ids via `allTextContents()`,
    // so that one option's "id" came back as `"Checked1.20.1-47.4.22"` —
    // never equal to its own `data-testid` suffix, which broke both a
    // seeded pick landing on index 0 (clicks a testid that doesn't exist)
    // and an explicit `loaderVersion` equal to index 0's real id (fails the
    // `offeredVersions.includes(...)` check with a false "not offered"
    // error). Confirmed live against the packaged app: Task 4's own smoke
    // test captured the raw contamination as
    // `offered=Checked1.20.1-47.4.22|1.20.1-47.4.21|...` before this fix.
    // The `data-testid` attribute this function reads is never touched by
    // that rendering — extracting from it directly (rather than from
    // `textContent`) makes the checkmark irrelevant regardless of which
    // index is selected.
    const preSelectedOptionTestId =
      "instance-creation-loader-version-option-1.20.1-47.4.22"
    const id = loaderVersionIdFromTestId(preSelectedOptionTestId)
    expect(id).toBe("1.20.1-47.4.22")
    expect(id).not.toMatch(/^Checked/)
  })

  it("throws a descriptive error on an unprefixed testid", () => {
    expect(() =>
      loaderVersionIdFromTestId("not-a-loader-version-option")
    ).toThrow(/prefix/i)
  })
})

describe("deriveLoaderVersionSeed", () => {
  it("is deterministic for a given (baseSeed, loader, version)", () => {
    const a = deriveLoaderVersionSeed(12345, "forge", "1.20.1")
    const b = deriveLoaderVersionSeed(12345, "forge", "1.20.1")
    expect(a).toBe(b)
  })

  it("gives different (loader, version) pairs their own draw at one base seed", () => {
    // Regression guard: before this existed, `pickSeededOption(offered,
    // baseSeed)` always consumed `mulberry32(baseSeed)`'s very first float —
    // every loader and every Minecraft version in a run landed on the same
    // relative percentile of its own offered list. Not asserting two
    // specific pairs differ (a coin flip on a 10-element list) but sampling
    // many pairs at one fixed base seed and requiring the picks to spread,
    // following the same style as `pickSeededOption`'s own spread test.
    const LOADERS: Loader[] = ["forge", "neoforge", "fabric", "quilt"]
    const VERSIONS = [
      "1.20.1",
      "1.16.5",
      "1.12.2",
      "1.7.10",
      "1.6.4",
      "26.2",
      "1.21.4",
      "1.19.2"
    ]
    const baseSeed = 469278827

    const picks = new Set<string>()
    for (const loader of LOADERS) {
      for (const version of VERSIONS) {
        const seed = deriveLoaderVersionSeed(baseSeed, loader, version)
        picks.add(pickSeededOption(OPTIONS, seed))
      }
    }
    expect(picks.size).toBeGreaterThan(5)
  })

  it("spreads across many base seeds too, for a fixed (loader, version)", () => {
    const picks = new Set(
      Array.from({ length: 30 }, (_, seed) =>
        pickSeededOption(
          OPTIONS,
          deriveLoaderVersionSeed(seed, "forge", "1.20.1")
        )
      )
    )
    expect(picks.size).toBeGreaterThan(5)
  })
})
