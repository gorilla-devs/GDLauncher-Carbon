import { describe, it, expect } from "vitest"
import { optionsEqual } from "./optionsEqual"

describe("optionsEqual", () => {
  it("the same array reference is equal", () => {
    const arr = ["a", "b"]
    expect(optionsEqual(arr, arr)).toBe(true)
  })

  it("different arrays with the same primitives in the same order are equal", () => {
    expect(optionsEqual(["a", "b", "c"], ["a", "b", "c"])).toBe(true)
  })

  it("different arrays with a different primitive are not equal", () => {
    expect(optionsEqual(["a", "b"], ["a", "c"])).toBe(false)
  })

  it("arrays of different lengths are not equal", () => {
    expect(optionsEqual(["a", "b"], ["a"])).toBe(false)
  })

  it("order matters — same items in a different order are not equal", () => {
    expect(optionsEqual(["a", "b"], ["b", "a"])).toBe(false)
  })

  it("without optionKey, structurally identical objects with different references are equal", () => {
    const prev = [{ id: 1, name: "one" }]
    const next = [{ id: 1, name: "one" }]
    expect(optionsEqual(prev, next)).toBe(true)
  })

  it("without optionKey, structurally different objects are not equal", () => {
    const prev = [{ id: 1, name: "one" }]
    const next = [{ id: 1, name: "renamed" }]
    expect(optionsEqual(prev, next)).toBe(false)
  })

  it("with optionKey, only the derived key is compared", () => {
    const prev = [{ id: 1, name: "one" }]
    const next = [{ id: 1, name: "renamed" }]
    expect(optionsEqual(prev, next, (o) => o.id.toString())).toBe(true)
  })

  it("with optionKey, a changed key is not equal", () => {
    const prev = [{ id: 1, name: "one" }]
    const next = [{ id: 2, name: "one" }]
    expect(optionsEqual(prev, next, (o) => o.id.toString())).toBe(false)
  })

  it("a circular-reference object falls back to not-equal instead of throwing", () => {
    const a: Record<string, unknown> = { id: 1 }
    a.self = a
    const b: Record<string, unknown> = { id: 1 }
    b.self = b
    expect(() => optionsEqual([a], [b])).not.toThrow()
    expect(optionsEqual([a], [b])).toBe(false)
  })

  it("two empty arrays are equal", () => {
    expect(optionsEqual([], [])).toBe(true)
  })
})
