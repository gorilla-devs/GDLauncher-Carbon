import { describe, expect, it } from "vitest"
import { textLinkClass } from "./TextLink"

describe("textLinkClass", () => {
  it("shares one base recipe", () => {
    for (const tone of ["bright", "muted"] as const) {
      const c = textLinkClass(tone).split(" ")
      for (const k of [
        "rounded",
        "text-sm",
        "underline",
        "underline-offset-2",
        "outline-none",
        "focus-visible:outline",
        "focus-visible:outline-2",
        "focus-visible:outline-offset-2",
        "focus-visible:outline-primary-500",
        "disabled:opacity-50",
        "disabled:cursor-not-allowed"
      ])
        expect(c).toContain(k)
    }
  })

  it("differs by tone", () => {
    expect(textLinkClass("bright")).toContain("text-lightSlate-100")
    expect(textLinkClass("muted")).toContain("text-lightSlate-700")
    expect(textLinkClass("muted")).not.toContain("text-lightSlate-100")
  })
})
