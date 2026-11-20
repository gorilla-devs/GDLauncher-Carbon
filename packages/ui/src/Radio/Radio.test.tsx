import { describe, expect, it, vi } from "vitest"
import { fireEvent, render, screen } from "solid-testing-library"
import { Radio } from "."

describe("Radio", () => {
  it("is a radio named by its label", () => {
    render(() => (
      <Radio value="a" checked onChange={() => {}}>
        Alpha
      </Radio>
    ))
    const radio = screen.getByRole("radio", { name: "Alpha" })
    expect((radio as HTMLInputElement).checked).toBe(true)
  })

  it("takes a description, kept out of its name", () => {
    // Download Java: a build's known issues, which its icon shows the mouse.
    render(() => (
      <Radio value="a" checked={false} aria-describedby="issues">
        8.0.412
        <span id="issues" hidden>
          Security flaw: CVE-2026-1
        </span>
      </Radio>
    ))
    expect(
      screen.getByRole("radio", {
        name: "8.0.412",
        description: "Security flaw: CVE-2026-1"
      })
    ).toBeTruthy()
  })

  it("renders its label once", () => {
    render(() => (
      <Radio value="a" checked={false}>
        <span>Alpha</span>
      </Radio>
    ))
    expect(screen.getAllByText("Alpha")).toHaveLength(1)
    expect(screen.getByRole("radio").childNodes).toHaveLength(0)
  })

  it("joins the radios that share its name into one group", () => {
    render(() => (
      <>
        <Radio name="build" value="a" checked>
          Alpha
        </Radio>
        <Radio name="build" value="b" checked={false}>
          Beta
        </Radio>
      </>
    ))
    const names = screen
      .getAllByRole("radio")
      .map((radio) => radio.getAttribute("name"))
    expect(names).toEqual(["build", "build"])
  })

  it("names the radios of a Radio.group as one group", () => {
    render(() => (
      <Radio.group
        value="a"
        options={[
          { value: "a", label: "Alpha" },
          { value: "b", label: "Beta" }
        ]}
      />
    ))
    const [first, second] = screen.getAllByRole("radio")
    expect(first.getAttribute("name")).toBeTruthy()
    expect(second.getAttribute("name")).toBe(first.getAttribute("name"))
  })

  it("picks from the keyboard", () => {
    const onChange = vi.fn()
    render(() => (
      <Radio value="b" checked={false} onChange={onChange}>
        Beta
      </Radio>
    ))
    // Space or an arrow key checks a native radio, as a click on it does.
    fireEvent.click(screen.getByRole("radio", { name: "Beta" }))
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenLastCalledWith("b")
  })

  it("picks once from a click on its label", () => {
    const onChange = vi.fn()
    render(() => (
      <Radio value="b" checked={false} onChange={onChange}>
        Beta
      </Radio>
    ))
    fireEvent.click(screen.getByText("Beta"))
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenLastCalledWith("b")
  })

  it("deselects once from a click when it allows that", () => {
    const onChange = vi.fn()
    render(() => (
      <Radio value="b" checked allowDeselect onChange={onChange}>
        Beta
      </Radio>
    ))
    fireEvent.click(screen.getByText("Beta"))
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenLastCalledWith("b")
  })

  it("doesn't pick from a click on a button inside that stops it", () => {
    // Download Java's known-issue icon sits inside a build's label.
    const onChange = vi.fn()
    render(() => (
      <Radio value="b" checked={false} onChange={onChange}>
        Beta
        <button type="button" onClick={(e) => e.stopPropagation()}>
          Known issues
        </button>
      </Radio>
    ))
    fireEvent.click(screen.getByRole("button", { name: "Known issues" }))
    expect(onChange).not.toHaveBeenCalled()
  })
})
