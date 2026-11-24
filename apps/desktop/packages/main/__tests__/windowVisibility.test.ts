import { describe, it, expect, vi } from "vitest"
import { EventEmitter } from "node:events"
import { forwardWindowVisibility } from "../windowVisibility"

function fake(visible: boolean, minimized: boolean) {
  const win: any = new EventEmitter()
  win.isVisible = vi.fn(() => visible)
  win.isMinimized = vi.fn(() => minimized)
  return win
}

describe("window visibility", () => {
  it("forwards window visibility", () => {
    const win = fake(true, false)
    const send = vi.fn()
    forwardWindowVisibility(win, send)
    win.emit("minimize")
    expect(send).toHaveBeenLastCalledWith(false)
    win.emit("hide")
    expect(send).toHaveBeenLastCalledWith(false)
    win.emit("restore")
    expect(send).toHaveBeenLastCalledWith(true)
    win.isMinimized.mockReturnValue(true)
    win.emit("show")
    expect(send).toHaveBeenLastCalledWith(false)
    expect(send).toHaveBeenCalledTimes(4)
  })
})
