import { describe, expect, it } from "vitest"
import { fireEvent, render, screen } from "solid-testing-library"
import { Collapsable } from "."

/** Whether `el` sits in an inert subtree. Solid sets the `inert` property,
 *  which Chromium reflects to the attribute; jsdom keeps it a plain one. */
const isInert = (el: Element) => {
  for (let node: Element | null = el; node; node = node.parentElement)
    if ((node as HTMLElement).inert) return true
  return false
}

describe("Collapsable", () => {
  it("toggles from a real button that says whether it is open", () => {
    render(() => (
      <Collapsable title="Advanced" defaultOpened={false}>
        <p>Inside</p>
      </Collapsable>
    ))

    const header = screen.getByRole("button", { name: "Advanced" })
    expect(header.getAttribute("type")).toBe("button")
    expect(header.getAttribute("aria-expanded")).toBe("false")

    fireEvent.click(header)
    expect(header.getAttribute("aria-expanded")).toBe("true")
  })

  it("keeps a closed section's content out of reach until it opens", () => {
    render(() => (
      <Collapsable title="Advanced" defaultOpened={false}>
        <button type="button">Inside</button>
      </Collapsable>
    ))

    const header = screen.getByRole("button", { name: "Advanced" })
    const inside = screen.getByText("Inside")
    expect(isInert(inside)).toBe(true)

    fireEvent.click(header)
    expect(isInert(inside)).toBe(false)

    fireEvent.click(header)
    expect(isInert(inside)).toBe(true)
  })

  it("leaves an open section's content in reach", () => {
    render(() => (
      <Collapsable title="Advanced">
        <button type="button">Inside</button>
      </Collapsable>
    ))

    expect(isInert(screen.getByText("Inside"))).toBe(false)
  })

  it("leaves room for focus outlines at an open section's edge", () => {
    render(() => (
      <Collapsable title="Advanced" defaultOpened={false}>
        <button type="button">Inside</button>
      </Collapsable>
    ))
    const content = screen.getByText("Inside").parentElement!
    // Closed: clipped at its edge, so nothing peeks out.
    expect(content.style.overflow).toBe("hidden")

    fireEvent.click(screen.getByRole("button", { name: "Advanced" }))
    expect(content.style.overflow).toBe("clip")
    expect(content.style.getPropertyValue("overflow-clip-margin")).toBe("4px")
  })

  it("keeps a custom header as given", () => {
    render(() => (
      <Collapsable
        title="Ignored"
        customHeader={(toggle) => <span onClick={toggle}>Custom</span>}
      >
        <p>Inside</p>
      </Collapsable>
    ))

    expect(screen.queryByRole("button")).toBeNull()
    expect(screen.getByText("Custom")).toBeTruthy()
  })
})
