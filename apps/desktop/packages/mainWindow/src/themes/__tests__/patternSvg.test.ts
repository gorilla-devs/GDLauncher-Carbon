// @vitest-environment jsdom
import { describe, it, expect } from "vitest"
import asset from "../../../public/assets/images/gdlauncher_pattern.svg?raw"
import { themedPatternSvg } from "../patternSvg"

// A painted rect as big as the picture would cover the ad column's colour,
// which a theme sets with its `ad-column-background` rule.
const paintedFullRects = (svg: SVGSVGElement) => {
  const [, , w, h] = svg.getAttribute("viewBox")!.split(/\s+/).map(Number)
  return [...svg.querySelectorAll("rect")].filter(
    (r) =>
      !r.closest("clipPath, mask, defs") &&
      Number(r.getAttribute("width")) >= w &&
      Number(r.getAttribute("height")) >= h &&
      r.getAttribute("fill") !== "none"
  )
}

describe("the ad column's logo pattern", () => {
  it("leaves no painted full-size rect in the real asset", () => {
    const svg = themedPatternSvg(asset)!
    expect(svg).not.toBeNull()
    expect(paintedFullRects(svg)).toEqual([])
    expect(svg.querySelectorAll(".pattern-fill").length).toBeGreaterThan(0)
    expect(svg.querySelectorAll(".pattern-bg").length).toBeGreaterThan(0)
    expect(svg.querySelector('[fill^="rgb(var(--pattern"]')).toBeNull()
    expect(svg.getAttribute("preserveAspectRatio")).toBe("xMidYMid slice")
    expect(svg.hasAttribute("width")).toBe(false)
  })

  it("finds the backdrop rect whatever order its attributes are in", () => {
    const svg = themedPatternSvg(
      `<svg viewBox="0 0 100 50" xmlns="http://www.w3.org/2000/svg">
        <rect fill="rgb(var(--pattern-background))" height="50" width="100" x="0"/>
        <rect width="100%" height="100%" fill="rgb(var(--pattern-background))"/>
        <path d="M0 0h10v10z" fill="rgb(var(--pattern-fill))"/>
        <path d="M2 2h4v4z" fill="rgb(var(--pattern-background))"/>
      </svg>`
    )!
    expect(paintedFullRects(svg)).toEqual([])
    const rects = [...svg.querySelectorAll("rect")]
    expect(rects.map((r) => r.getAttribute("fill"))).toEqual(["none", "none"])
    expect(svg.querySelectorAll(".pattern-fill")).toHaveLength(1)
    expect(svg.querySelectorAll(".pattern-bg")).toHaveLength(1)
  })
})
