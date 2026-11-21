// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest"
import { addAccountPath, animatesOpen, watchOpenStrip } from "./behaviour"

const setup = () => {
  document.body.innerHTML = `
    <input id="outside" />
    <div id="root"><button id="inside">a</button><button id="other">b</button></div>`
  const root = document.getElementById("root")!
  const close = vi.fn()
  const stop = watchOpenStrip(root, close)
  return { root, close, stop }
}

const key = (target: Element, k: string) => {
  const e = new KeyboardEvent("keydown", {
    key: k,
    bubbles: true,
    cancelable: true
  })
  target.dispatchEvent(e)
  return e
}

describe("the open account strip", () => {
  let stop: () => void = () => {}
  afterEach(() => stop())

  it("Esc in the strip closes it and gives focus back", () => {
    const s = setup()
    stop = s.stop
    const inside = document.getElementById("inside")!
    inside.focus()
    expect(key(inside, "Escape").defaultPrevented).toBe(true)
    expect(s.close).toHaveBeenCalledWith(true)
  })

  it("Esc elsewhere is left alone", () => {
    const s = setup()
    stop = s.stop
    const outside = document.getElementById("outside")!
    outside.focus()
    s.close.mockClear()
    const e = key(outside, "Escape")
    expect(e.defaultPrevented).toBe(false)
    expect(s.close).not.toHaveBeenCalledWith(true)
  })

  it("focus leaving the strip closes it", () => {
    const s = setup()
    stop = s.stop
    document.getElementById("inside")!.focus()
    document.getElementById("outside")!.focus()
    expect(s.close).toHaveBeenCalledWith(false)
  })

  it("focus moving inside the strip keeps it open", () => {
    const s = setup()
    stop = s.stop
    document.getElementById("inside")!.focus()
    document.getElementById("other")!.focus()
    expect(s.close).not.toHaveBeenCalled()
  })

  it("a press outside closes it, one inside doesn't", () => {
    const s = setup()
    stop = s.stop
    document
      .getElementById("inside")!
      .dispatchEvent(new Event("pointerdown", { bubbles: true }))
    expect(s.close).not.toHaveBeenCalled()
    document
      .getElementById("outside")!
      .dispatchEvent(new Event("pointerdown", { bubbles: true }))
    expect(s.close).toHaveBeenCalledWith(false)
  })

  it("stops listening once it is closed", () => {
    const s = setup()
    s.stop()
    document.getElementById("inside")!.focus()
    key(document.getElementById("inside")!, "Escape")
    document
      .getElementById("outside")!
      .dispatchEvent(new Event("pointerdown", { bubbles: true }))
    expect(s.close).not.toHaveBeenCalled()
  })
})

describe("adding an account", () => {
  it("returns to the page it was opened on, encoded", () => {
    expect(addAccountPath({ pathname: "/library", search: "?a=b&c=d" })).toBe(
      "/?addMicrosoftAccount=true&returnTo=%2Flibrary%3Fa%3Db%26c%3Dd"
    )
    expect(addAccountPath({ pathname: "/settings", search: "" })).toBe(
      "/?addMicrosoftAccount=true&returnTo=%2Fsettings"
    )
  })
})

describe("the opening animation", () => {
  it("plays only while motion runs (not with reduced motion, app or OS)", () => {
    expect(animatesOpen("run")).toBe(true)
    expect(animatesOpen("pause")).toBe(false)
    expect(animatesOpen("off")).toBe(false)
  })
})
