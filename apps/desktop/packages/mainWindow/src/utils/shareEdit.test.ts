import { describe, expect, it } from "vitest"
import { listedShare, shareChanges, shareChangesApplied } from "./shareEdit"

const share = (title: string | null, maxDownloads: number | null) =>
  ({ shareCode: "abc", title, maxDownloads }) as any

describe("shareChanges", () => {
  it("leaves out what the form did not change", () => {
    expect(
      shareChanges(share("Skyblock", 100), {
        title: "Skyblock",
        maxDownloads: "100"
      })
    ).toEqual({})
    expect(
      shareChanges(share(null, null), { title: "  ", maxDownloads: "" })
    ).toEqual({})
  })

  it("asks to remove an emptied limit and an emptied title", () => {
    expect(
      shareChanges(share("Skyblock", 100), { title: "", maxDownloads: "" })
    ).toEqual({ title: "", maxDownloads: null })
  })

  it("sends only the field that changed", () => {
    expect(
      shareChanges(share("Skyblock", 100), {
        title: " Skyblock 2 ",
        maxDownloads: "100"
      })
    ).toEqual({ title: "Skyblock 2" })
    expect(
      shareChanges(share("Skyblock", null), {
        title: "Skyblock",
        maxDownloads: "25"
      })
    ).toEqual({ maxDownloads: 25 })
  })
})

describe("shareChangesApplied", () => {
  it("is false while the limit asked to be removed is still listed", () => {
    expect(
      shareChangesApplied(share("Skyblock", 100), { maxDownloads: null })
    ).toBe(false)
    expect(
      shareChangesApplied(share("Skyblock", null), { maxDownloads: null })
    ).toBe(true)
  })

  it("is false while the title asked to be removed is still listed", () => {
    expect(shareChangesApplied(share("Skyblock", 100), { title: "" })).toBe(
      false
    )
    expect(shareChangesApplied(share("", 100), { title: "" })).toBe(true)
    expect(shareChangesApplied(share(null, 100), { title: "" })).toBe(true)
  })

  it("checks only what was asked", () => {
    expect(shareChangesApplied(share("Other", 7), {})).toBe(true)
    expect(shareChangesApplied(share("Other", 25), { maxDownloads: 25 })).toBe(
      true
    )
    expect(shareChangesApplied(share("Other", 24), { maxDownloads: 25 })).toBe(
      false
    )
    expect(
      shareChangesApplied(share("New", 25), { title: "New", maxDownloads: 5 })
    ).toBe(false)
  })
})

describe("listedShare", () => {
  const pages = [
    { items: [share("One", 1)], totalCount: 2, limit: 1, offset: 0 },
    {
      items: [{ ...share("Two", 2), shareCode: "def" }],
      totalCount: 2,
      limit: 1,
      offset: 1
    }
  ]

  it("finds the share on any page that was read again", () => {
    expect(listedShare({ isError: false, data: { pages } }, "def")?.title).toBe(
      "Two"
    )
  })

  it("has no answer when the list could not be read again", () => {
    expect(listedShare({ isError: true, data: { pages } }, "abc")).toBe(
      undefined
    )
    expect(listedShare({ isError: false }, "abc")).toBe(undefined)
  })

  it("has no answer when the share is no longer listed", () => {
    expect(listedShare({ isError: false, data: { pages } }, "gone")).toBe(
      undefined
    )
  })
})
