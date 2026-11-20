import { describe, expect, it } from "vitest"
import { render } from "solid-testing-library"
import { Button } from "./Button"
import { Checkbox } from "./Checkbox"
import { Input } from "./Input"
import { Radio } from "./Radio"
import { Switch } from "./Switch"
import { TextArea } from "./TextArea"

// A ring in the primary colour vanishes on a primary button, so every shared
// control shows keyboard focus as an offset outline instead, or an inset one
// where its container clips.
const expectOutline = (
  el: Element,
  prefix: string,
  offset = "outline-offset-2"
) => {
  const classes = el.className.split(/\s+/)
  for (const cls of ["outline", "outline-2", offset, "outline-primary-500"]) {
    expect(classes).toContain(`${prefix}${cls}`)
  }
  expect(el.className).not.toContain("ring-primary-500")
}
const expectOffsetOutline = (el: Element, prefix: string) =>
  expectOutline(el, prefix)

describe("keyboard focus", () => {
  it("outlines every Button variant with an offset", () => {
    const { getAllByRole } = render(() => (
      <>
        <Button type="primary">a</Button>
        <Button type="secondary">b</Button>
        <Button type="text">c</Button>
      </>
    ))
    const buttons = getAllByRole("button")
    expect(buttons).toHaveLength(3)
    for (const b of buttons) expectOffsetOutline(b, "focus-visible:")
  })

  it("outlines a TextArea", () => {
    const { container } = render(() => <TextArea />)
    expectOffsetOutline(container.querySelector("textarea")!, "focus-visible:")
  })

  it("outlines an Input's container when its field has focus", () => {
    const { container } = render(() => <Input />)
    const box = container.querySelector("[class*='has-[:focus-visible]']")!
    expectOffsetOutline(box, "has-[:focus-visible]:")
  })

  it("outlines a Switch's visible track from its hidden input", () => {
    const { container } = render(() => <Switch />)
    const input = container.querySelector("input")!
    expect(input.className).toContain("peer")
    expectOffsetOutline(input.nextElementSibling!, "peer-focus-visible:")
  })

  it("outlines a Checkbox's box from its input", () => {
    const { container } = render(() => (
      <Checkbox checked={false} onChange={() => {}} />
    ))
    const input = container.querySelector("input")!
    expect(input.className).toContain("peer")
    expectOffsetOutline(input.nextElementSibling!, "peer-focus-visible:")
  })

  it("outlines a Radio's row inside its edge, which scrolling lists clip", () => {
    const { container } = render(() => <Radio value="a" checked={false} />)
    // From its radio only: a button inside the label (a known-issue icon)
    // has an outline of its own.
    expectOutline(
      container.querySelector("label")!,
      "has-[input:focus-visible]:",
      "-outline-offset-2"
    )
  })
})
