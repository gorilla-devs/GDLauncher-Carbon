// @vitest-environment jsdom
import { describe, expect, it } from "vitest"
import { focusWasDropped } from "./dialogFocus"

describe("focusWasDropped", () => {
  const primary = document.createElement("button")
  const cancel = document.createElement("button")
  const dismiss = document.createElement("button")
  const own = [primary, cancel]

  it("counts focus on the body or nowhere as dropped", () => {
    expect(focusWasDropped(document.body, document.body, own)).toBe(true)
    expect(focusWasDropped(null, document.body, own)).toBe(true)
  })

  it("counts focus on the dialog's own buttons as dropped", () => {
    expect(focusWasDropped(primary, document.body, own)).toBe(true)
    expect(focusWasDropped(cancel, document.body, own)).toBe(true)
  })

  it("leaves focus alone when it went somewhere else on purpose", () => {
    expect(focusWasDropped(dismiss, document.body, own)).toBe(false)
  })

  it("ignores buttons that never rendered", () => {
    expect(focusWasDropped(dismiss, document.body, [undefined])).toBe(false)
  })
})
