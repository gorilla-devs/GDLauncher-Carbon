import { afterEach, describe, expect, it } from "vitest"
import { render, screen, cleanup } from "solid-testing-library"
import { createSignal } from "solid-js"
import { Button } from "./Button"
import { Badge } from "./Badge"
import { Checkbox } from "./Checkbox"
import { News } from "./News"
import { Slider, overlappingLabels } from "./Slider"
import { Toaster, toast } from "./Sonner"

// Theme hooks on the slider, the News slideshow and a checkbox's dash, the
// slider's label overlap, and which parts put words on an accent fill. The
// colours themselves are checked in the app's e2e, where the CSS applies.

afterEach(() => {
  cleanup()
  document.body.innerHTML = ""
})

const classes = (el: Element | null) => [...(el?.classList ?? [])]

describe("the slider", () => {
  const marks = { 1024: "1 GB", 2048: "2 GB", 4096: "4 GB", 8192: "8 GB" }

  it("marks its track, fill, thumb, marks and labels", () => {
    const { container } = render(() => (
      <Slider min={1024} max={8192} value={2048} marks={marks} />
    ))
    for (const hook of [
      "slider",
      "slider.track",
      "slider.fill",
      "slider.thumb"
    ]) {
      expect(
        container.querySelectorAll(`[data-gdl="${hook}"]`),
        hook
      ).toHaveLength(1)
    }
    expect(container.querySelectorAll('[data-gdl="slider.mark"]')).toHaveLength(
      4
    )
    const labels = container.querySelectorAll('[data-gdl="slider.label"]')
    expect([...labels].map((l) => l.textContent)).toEqual([
      "1 GB",
      "2 GB",
      "4 GB",
      "8 GB"
    ])
  })

  it("leaves the knob's resting colour to the stylesheet, not a utility", () => {
    const { container } = render(() => <Slider min={0} max={10} value={5} />)
    const thumb = container.querySelector('[data-gdl="slider.thumb"]')
    expect(classes(thumb).some((c) => c.startsWith("bg-lightSlate-50/"))).toBe(
      false
    )
  })

  it("hides a label that would run into the one before it", () => {
    // "1 GB" and "2 GB" a few px apart, as on a machine with a lot of memory.
    expect(
      overlappingLabels([
        { start: 32, end: 58 },
        { start: 52, end: 78 },
        { start: 100, end: 126 },
        { start: 300, end: 326 }
      ])
    ).toEqual([false, true, false, false])
  })

  it("measures from the last label kept, and keeps labels that just fit", () => {
    expect(
      overlappingLabels([
        { start: 0, end: 30 },
        { start: 20, end: 50 },
        { start: 34, end: 60 },
        { start: 62, end: 90 }
      ])
    ).toEqual([false, true, false, true])
  })

  it("ignores empty labels", () => {
    expect(
      overlappingLabels([
        { start: 0, end: 30 },
        { start: 10, end: 10 },
        { start: 40, end: 60 }
      ])
    ).toEqual([false, false, false])
  })
})

describe("the News slideshow", () => {
  const slides = [
    {
      image: "a.png",
      title: "First story",
      description: "Its summary",
      date: "2026-11-20"
    },
    {
      image: "b.png",
      title: "Second story",
      description: "Another summary",
      date: "2026-11-19"
    }
  ]

  it("marks the slideshow, its slides, words and controls", () => {
    const { container } = render(() => (
      <News slides={slides} disableAutoRotation />
    ))
    const count = (hook: string) =>
      container.querySelectorAll(`[data-gdl~="${hook}"]`).length
    expect(count("news.slideshow")).toBe(1)
    expect(count("news.slideshow.prev")).toBe(1)
    expect(count("news.slideshow.next")).toBe(1)
    expect(count("news.slideshow.dots")).toBe(1)
    expect(count("news.slideshow.dot")).toBe(2)
    expect(count("news.slideshow.dot.active")).toBe(1)
    // The two slides, plus a copy at each end for the endless loop.
    expect(count("news.slide")).toBe(4)
    for (const part of ["veil", "title", "text", "date"]) {
      expect(count(`news.slide.${part}`), part).toBe(4)
    }
  })

  it("keeps a fixed dark veil, and leaves the words' colours to the stylesheet", () => {
    const { container } = render(() => (
      <News slides={slides} disableAutoRotation />
    ))
    const first = (hook: string) =>
      container.querySelector(`[data-gdl="${hook}"]`)
    expect(classes(first("news.slide.veil"))).toContain(
      "bg-[rgb(29_32_40/0.7)]"
    )
    for (const hook of [
      "news.slide.title",
      "news.slide.text",
      "news.slide.date"
    ]) {
      expect(
        classes(first(hook)).some((c) =>
          /^text-(white|black|\[|lightSlate|darkSlate|primary)/.test(c)
        ),
        hook
      ).toBe(false)
    }
  })
})

describe("words on accent fills", () => {
  it("a primary button reads --gdl-on-primary, a tinted one doesn't", () => {
    render(() => (
      <>
        <Button>main</Button>
        <Button type="glow">glow</Button>
        <Button variant="red">red</Button>
        <Button type="secondary">second</Button>
      </>
    ))
    const cls = (text: string) =>
      classes(screen.getByText(text).closest("button"))
    expect(cls("main")).toContain("gdl-on-primary")
    expect(cls("main")).not.toContain("text-lightSlate-50")
    expect(cls("glow")).toContain("gdl-on-primary")
    expect(cls("red")).toContain("text-lightSlate-50")
    expect(cls("red")).not.toContain("gdl-on-primary")
    expect(cls("second")).not.toContain("gdl-on-primary")
  })

  it("a disabled primary button reads --gdl-on-primary-disabled", () => {
    render(() => <Button disabled>off</Button>)
    const cls = classes(screen.getByText("off").closest("button"))
    expect(cls).toContain("gdl-on-primary-disabled")
    expect(cls).not.toContain("text-[#404759]")
  })

  it("a gold badge reads --gdl-on-primary unless a caller set a text colour", () => {
    render(() => (
      <>
        <Badge>gold</Badge>
        <Badge class="text-red-500">own</Badge>
        <Badge variant="secondary">plain</Badge>
      </>
    ))
    expect(classes(screen.getByText("gold"))).toContain("gdl-on-primary")
    expect(classes(screen.getByText("own"))).not.toContain("gdl-on-primary")
    expect(classes(screen.getByText("plain"))).not.toContain("gdl-on-primary")
  })

  it("a ticked box's mark reads --gdl-on-primary", () => {
    const { container } = render(() => <Checkbox checked />)
    expect(
      classes(container.querySelector('[data-gdl="checkbox.mark"]'))
    ).toContain("gdl-on-primary")
  })

  it("a toast's action button reads it too", async () => {
    render(() => <Toaster />)
    toast("with action", { action: { label: "Go", onClick: () => {} } })
    const go = await screen.findByText("Go")
    expect((go.closest("button") ?? go).getAttribute("style")).toContain(
      "--gdl-on-primary"
    )
  })
})

describe("a checkbox's dash", () => {
  it("has a hook while the box shows it", () => {
    const [checked, setChecked] = createSignal(false)
    const { container } = render(() => (
      <Checkbox indeterminate checked={checked()} />
    ))
    expect(container.querySelector('[data-gdl="checkbox.dash"]')).not.toBeNull()
    setChecked(true)
    expect(container.querySelector('[data-gdl="checkbox.dash"]')).toBeNull()
  })
})
