import { describe, expect, it } from "vitest"
import {
  curseforgeChannel,
  modrinthChannel,
  newestByDate,
  newestUpdateCandidate,
  type ResolutionCandidate
} from "./resolution.js"

function candidate(
  over: Partial<ResolutionCandidate> & { id: string; datePublished: string }
): ResolutionCandidate {
  return {
    channel: "stable",
    gameVersions: ["1.20.1"],
    loaders: ["fabric"],
    ...over
  }
}

describe("newestByDate", () => {
  it("returns the newest regardless of input order", () => {
    const older = candidate({ id: "a", datePublished: "2024-01-01T00:00:00Z" })
    const newer = candidate({ id: "b", datePublished: "2024-06-01T00:00:00Z" })
    expect(newestByDate([older, newer]).id).toBe("b")
    expect(newestByDate([newer, older]).id).toBe("b")
  })

  it("ignores channel", () => {
    const stable = candidate({ id: "a", datePublished: "2024-01-01T00:00:00Z" })
    const beta = candidate({
      id: "b",
      datePublished: "2024-06-01T00:00:00Z",
      channel: "beta"
    })
    expect(newestByDate([stable, beta]).id).toBe("b")
  })

  it("throws on an empty list", () => {
    expect(() => newestByDate([])).toThrow(/empty/i)
  })

  it("throws when the newest date is shared", () => {
    const a = candidate({ id: "a", datePublished: "2024-06-01T00:00:00Z" })
    const b = candidate({ id: "b", datePublished: "2024-06-01T00:00:00Z" })
    expect(() => newestByDate([a, b])).toThrow(/ambiguous/i)
  })
})

describe("newestUpdateCandidate", () => {
  it("prefers the newest stable over a newer beta", () => {
    const stable = candidate({ id: "a", datePublished: "2024-01-01T00:00:00Z" })
    const beta = candidate({
      id: "b",
      datePublished: "2024-06-01T00:00:00Z",
      channel: "beta"
    })
    expect(newestUpdateCandidate([stable, beta]).id).toBe("a")
  })

  it("falls back to beta when no stable exists", () => {
    const alpha = candidate({
      id: "a",
      datePublished: "2024-06-01T00:00:00Z",
      channel: "alpha"
    })
    const beta = candidate({
      id: "b",
      datePublished: "2024-01-01T00:00:00Z",
      channel: "beta"
    })
    expect(newestUpdateCandidate([alpha, beta]).id).toBe("b")
  })

  it("falls back to alpha when neither stable nor beta exists", () => {
    const a = candidate({
      id: "a",
      datePublished: "2024-06-01T00:00:00Z",
      channel: "alpha"
    })
    expect(newestUpdateCandidate([a]).id).toBe("a")
  })

  it("throws on an empty list", () => {
    expect(() => newestUpdateCandidate([])).toThrow(/empty/i)
  })

  it("throws when the winning channel's newest date is shared", () => {
    const a = candidate({ id: "a", datePublished: "2024-06-01T00:00:00Z" })
    const b = candidate({ id: "b", datePublished: "2024-06-01T00:00:00Z" })
    const beta = candidate({
      id: "c",
      datePublished: "2024-07-01T00:00:00Z",
      channel: "beta"
    })
    expect(() => newestUpdateCandidate([a, b, beta])).toThrow(/ambiguous/i)
  })
})

describe("channel mapping", () => {
  it("maps modrinth version types", () => {
    expect(modrinthChannel("release")).toBe("stable")
    expect(modrinthChannel("beta")).toBe("beta")
    expect(modrinthChannel("alpha")).toBe("alpha")
  })

  it("maps curseforge release types", () => {
    expect(curseforgeChannel("stable")).toBe("stable")
    expect(curseforgeChannel("beta")).toBe("beta")
    expect(curseforgeChannel("alpha")).toBe("alpha")
  })

  it("throws on unrecognised inputs rather than defaulting", () => {
    expect(() => modrinthChannel("nightly")).toThrow()
    expect(() => curseforgeChannel("nightly")).toThrow()
  })

  it("throws on a numeric releaseType rather than coercing it", () => {
    // The Rust `ReleaseType` enum is `#[repr(i32)]`, so reading the source
    // suggests an integer reaches the renderer. Task 1 confirmed live that
    // it does not — it arrives as a string. If that ever changes, this must
    // fail loudly rather than silently mapping to an unintended channel.
    //
    // A bare `.toThrow()` can't tell this `typeof` guard apart from
    // `(1).toLowerCase` throwing its own native `TypeError` regardless —
    // pinned to the guard's own message so this only passes when the guard
    // itself actually fired.
    expect(() => curseforgeChannel(1 as unknown as string)).toThrow(
      /expected a string/i
    )
  })
})
