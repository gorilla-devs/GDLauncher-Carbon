import { describe, expect, it } from "vitest"
import { clampMenuX } from "./clamp"

describe("clampMenuX", () => {
  it("keeps the menu clear of the ad column", () => {
    expect(clampMenuX(1100, 160, 1160)).toBe(1000)
  })
  it("leaves menus that fit alone", () => {
    expect(clampMenuX(200, 160, 1160)).toBe(200)
  })
  it("never goes negative", () => {
    expect(clampMenuX(-5, 160, 1160)).toBe(0)
  })
})
