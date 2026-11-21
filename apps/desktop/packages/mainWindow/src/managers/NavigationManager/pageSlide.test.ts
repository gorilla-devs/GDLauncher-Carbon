// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest"
import { createPageSlides, PAGE_SLIDE_CLASS } from "./pageSlide"

const RIGHT = "slide-right-transition"
const LEFT = "slide-left-transition"
const html = () => document.documentElement.classList

describe("page slides", () => {
  beforeEach(() => {
    document.body.className = ""
    html().remove(PAGE_SLIDE_CLASS)
    document.body.innerHTML = '<div id="gdl-page"></div>'
  })
  const page = () => document.getElementById("gdl-page")!

  it("only the page box slides, and the rest is told so", () => {
    const end = createPageSlides(document, "gdl-page")(RIGHT)
    expect(page().classList.contains(RIGHT)).toBe(true)
    expect(document.body.classList.contains(RIGHT)).toBe(false)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(true)
    end()
    expect(page().classList.contains(RIGHT)).toBe(false)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(false)
  })

  it("without the page box the body slides, as before", () => {
    document.body.innerHTML = ""
    const end = createPageSlides(document, "gdl-page")(LEFT)
    expect(document.body.classList.contains(LEFT)).toBe(true)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(false)
    end()
    expect(document.body.classList.contains(LEFT)).toBe(false)
  })

  it("a slide that ends after a newer one started leaves the newer one alone", () => {
    const slide = createPageSlides(document, "gdl-page")
    const endFirst = slide(RIGHT)
    const endSecond = slide(LEFT)
    expect(page().classList.contains(RIGHT), "the stale class goes").toBe(false)
    expect(page().classList.contains(LEFT)).toBe(true)
    endFirst()
    expect(page().classList.contains(LEFT)).toBe(true)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(true)
    endSecond()
    expect(page().classList.contains(LEFT)).toBe(false)
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(false)
  })

  it("a navigation without a slide changes nothing", () => {
    const end = createPageSlides(document, "gdl-page")(undefined)
    expect(page().className).toBe("")
    expect(html().contains(PAGE_SLIDE_CLASS)).toBe(false)
    end()
  })
})
