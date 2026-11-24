// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest"
import { escapeClosesModal } from "./dialogEscape"

const mount = (html: string) => {
  const template = document.createElement("template")
  template.innerHTML = html.trim()
  const el = template.content.firstElementChild as HTMLElement
  document.body.append(el)
  return el
}

const byId = (id: string) => document.getElementById(id)!

/** Presses a key on `target` and asks, as the manager's document listener
 *  does after the target's own handlers, whether it closes the modal. */
const press = (
  target: Element,
  init: KeyboardEventInit = {},
  { cancelOnTarget = false } = {}
) => {
  let result: boolean | undefined
  const onTarget = (e: Event) => {
    if (cancelOnTarget) e.preventDefault()
  }
  const onDocument = (e: KeyboardEvent) => {
    result = escapeClosesModal(e, document)
  }
  target.addEventListener("keydown", onTarget)
  document.addEventListener("keydown", onDocument)
  target.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Escape",
      bubbles: true,
      cancelable: true,
      ...init
    })
  )
  target.removeEventListener("keydown", onTarget)
  document.removeEventListener("keydown", onDocument)
  return result
}

/** The dialog with a control in it, where focus is. */
const dialogWithButton = () => {
  mount(
    `<div role="dialog" tabindex="-1"><button id="inside">inside</button></div>`
  )
  return byId("inside")
}

afterEach(() => {
  document.body.innerHTML = ""
})

describe("escapeClosesModal", () => {
  it("closes on Escape", () => {
    expect(press(dialogWithButton())).toBe(true)
  })

  it("ignores other keys", () => {
    expect(press(dialogWithButton(), { key: "Enter" })).toBe(false)
  })

  it("leaves Escape to a text field that is composing", () => {
    expect(press(dialogWithButton(), { isComposing: true })).toBe(false)
  })

  it("leaves Escape to an open listbox or menu", () => {
    const inside = dialogWithButton()
    mount(`<div role="listbox"></div>`)
    expect(press(inside)).toBe(false)
    document.querySelector('[role="listbox"]')!.setAttribute("role", "menu")
    expect(press(inside)).toBe(false)
  })

  it("leaves Escape to an open popover", () => {
    const inside = dialogWithButton()
    mount(
      `<div data-popper-positioner><div role="dialog" data-expanded><button>in popover</button></div></div>`
    )
    expect(press(inside)).toBe(false)
  })

  it("closes past a popover on its way out", () => {
    const inside = dialogWithButton()
    mount(
      `<div data-popper-positioner><div role="dialog" data-closed></div></div>`
    )
    expect(press(inside)).toBe(true)
  })

  it("closes past an open tooltip, as before", () => {
    const inside = dialogWithButton()
    mount(
      `<div data-popper-positioner><div role="tooltip" data-expanded>Add Java</div></div>`
    )
    expect(press(inside)).toBe(true)
  })

  it("leaves Escape to a control that cancelled it", () => {
    expect(press(dialogWithButton(), {}, { cancelOnTarget: true })).toBe(false)
  })

  it("closes from a tab, whose Kobalte list cancels every Escape", () => {
    // New Instance: Kobalte's selectable collection calls preventDefault on
    // Escape, though tabs never clear their selection.
    mount(
      `<div role="dialog" tabindex="-1"><div role="tablist"><button role="tab" id="custom">Custom</button></div></div>`
    )
    expect(press(byId("custom"), {}, { cancelOnTarget: true })).toBe(true)
  })
})
