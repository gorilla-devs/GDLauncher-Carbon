import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { fireEvent, render, screen } from "solid-testing-library"
import { Checkbox } from "."

// jsdom has no pointer capture; the press handling needs it.
beforeEach(() => {
  HTMLElement.prototype.setPointerCapture = () => {}
  HTMLElement.prototype.releasePointerCapture = () => {}
  HTMLElement.prototype.hasPointerCapture = () => true
})
afterEach(() => {
  // @ts-expect-error put back as jsdom has it: missing
  delete HTMLElement.prototype.setPointerCapture
  // @ts-expect-error see above
  delete HTMLElement.prototype.releasePointerCapture
  // @ts-expect-error see above
  delete HTMLElement.prototype.hasPointerCapture
})

const checkbox = (name?: string) =>
  screen.getByRole<HTMLInputElement>("checkbox", name ? { name } : {})

describe("Checkbox", () => {
  it("is a checkbox named by its label", () => {
    render(() => (
      <Checkbox checked={false} onChange={() => {}}>
        <span>Show all</span>
      </Checkbox>
    ))
    const box = checkbox("Show all")
    expect(box.checked).toBe(false)
    expect(box.disabled).toBe(false)
  })

  it("takes a name when its label sits elsewhere", () => {
    render(() => (
      <Checkbox
        checked={false}
        aria-label="Select Sodium"
        onChange={() => {}}
      />
    ))
    expect(checkbox("Select Sodium")).toBeTruthy()
  })

  it("toggles from the keyboard", () => {
    const onChange = vi.fn()
    render(() => (
      <Checkbox checked={false} onChange={onChange}>
        <span>Don't ask again</span>
      </Checkbox>
    ))
    const box = checkbox("Don't ask again")
    // Space on a native checkbox is its activation: a click on it.
    fireEvent.click(box)
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenLastCalledWith(true)
    expect(box.checked).toBe(true)

    fireEvent.click(box)
    expect(onChange).toHaveBeenLastCalledWith(false)
    expect(box.checked).toBe(false)
  })

  it("toggles once from a press on its label", () => {
    const onChange = vi.fn()
    render(() => (
      <Checkbox checked={false} onChange={onChange}>
        <span>Don't ask again</span>
      </Checkbox>
    ))
    const label = screen.getByText("Don't ask again")
    fireEvent.pointerDown(label)
    fireEvent.pointerUp(label)
    fireEvent.click(label)
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenLastCalledWith(true)
    expect(checkbox().checked).toBe(true)
  })

  it("follows its checked prop", () => {
    const [checked, setChecked] = createSignal(false)
    render(() => (
      <Checkbox checked={checked()} onChange={() => {}}>
        <span>Show all</span>
      </Checkbox>
    ))
    setChecked(true)
    expect(checkbox().checked).toBe(true)
  })

  it("says when it is partly checked", () => {
    render(() => <Checkbox checked={false} indeterminate onChange={() => {}} />)
    expect(checkbox().indeterminate).toBe(true)
  })

  it("can't be changed while disabled", () => {
    render(() => (
      <Checkbox checked={false} disabled onChange={() => {}}>
        <span>Don't ask again</span>
      </Checkbox>
    ))
    expect(checkbox("Don't ask again").disabled).toBe(true)
  })

  it("is only a picture without onChange, for a parent that handles the click", () => {
    render(() => <Checkbox checked />)
    expect(screen.queryByRole("checkbox")).toBeNull()
  })
})
