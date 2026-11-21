// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest"
import { createPageSlides, PAGE_SLIDE_CLASS } from "./pageSlide"

const RIGHT = "slide-right-transition"
const LEFT = "slide-left-transition"
const html = () => document.documentElement.classList

// The classes go on <html>, which outlives the navigation, so the CSS can
// name the old page's panel and the new page's, though they are different
// elements (index.html).
describe("page slides", () => {
  beforeEach(() => {
    document.documentElement.className = ""
    document.body.className = ""
  })

  it("marks the slide and its direction on <html>, then clears them", () => {
    const end = createPageSlides(document)(RIGHT)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(true)
    expect(html().contains(RIGHT)).toBe(true)
    expect(document.body.className).toBe("")
    end()
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(false)
    expect(html().contains(RIGHT)).toBe(false)
  })

  it("a slide that ends after a newer one started leaves the newer one alone", () => {
    const slide = createPageSlides(document)
    const endFirst = slide(RIGHT)
    const endSecond = slide(LEFT)
    expect(html().contains(RIGHT), "the stale direction goes").toBe(false)
    expect(html().contains(LEFT)).toBe(true)
    endFirst()
    expect(html().contains(LEFT)).toBe(true)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(true)
    endSecond()
    expect(html().contains(LEFT)).toBe(false)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(false)
  })

  it("a navigation without a slide changes nothing", () => {
    const end = createPageSlides(document)(undefined)
    expect(document.documentElement.className).toBe("")
    end()
  })
})
