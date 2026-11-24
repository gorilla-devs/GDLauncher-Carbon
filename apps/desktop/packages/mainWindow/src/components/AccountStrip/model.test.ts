import { describe, it, expect } from "vitest"
import { needsAttention, stripAccounts } from "./model"

const acct = (uuid: string) => ({ uuid })

describe("the account strip's accounts", () => {
  it("the active account is never a face", () => {
    const [a, b, c] = [acct("a"), acct("b"), acct("c")]
    expect(stripAccounts([a, b, c], "b")).toEqual({
      active: b,
      faces: [a, c],
      more: 0
    })
  })

  it("past three faces the rest are counted", () => {
    const all = ["a", "b", "c", "d", "e", "f"].map(acct)
    const { active, faces, more } = stripAccounts(all, "d")
    expect(active).toBe(all[3])
    expect(faces).toEqual([all[0], all[1], all[2]])
    expect(more).toBe(2)
  })

  it("no accounts", () => {
    expect(stripAccounts([], "x")).toEqual({
      active: undefined,
      faces: [],
      more: 0
    })
  })

  it("an unknown active uuid", () => {
    const [a, b] = [acct("a"), acct("b")]
    expect(stripAccounts([a, b], "zzz")).toEqual({
      active: undefined,
      faces: [a, b],
      more: 0
    })
  })

  it("attention", () => {
    expect(needsAttention("ok")).toBeNull()
    expect(needsAttention("expired")).toBe("warning")
    expect(needsAttention("refreshing")).toBe("warning")
    expect(needsAttention("invalid")).toBe("error")
  })
})
