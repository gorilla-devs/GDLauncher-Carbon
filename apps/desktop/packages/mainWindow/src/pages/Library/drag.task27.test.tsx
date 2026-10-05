import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { DragProvider, useDragContext } from "./DragContext"
let drag: ReturnType<typeof useDragContext>,
  root: HTMLDivElement,
  dispose: (() => void) | undefined
let frames: FrameRequestCallback[], timeouts: (() => void)[]
const point = (x: number) => ({ clientX: x, clientY: 100 }) as PointerEvent
const handler = vi.fn()
beforeEach(() => {
  vi.useFakeTimers()
  frames = []
  timeouts = []
  handler.mockReset()
  const timer = globalThis.setTimeout
  vi.spyOn(globalThis, "setTimeout").mockImplementation(((
    fn: any,
    delay: any,
    ...args: any[]
  ) => {
    timeouts.push(() => fn(...args))
    return timer(fn, delay, ...args)
  }) as any)
  vi.stubGlobal("requestAnimationFrame", (fn: FrameRequestCallback) => {
    frames.push(fn)
    return frames.length
  })
  vi.stubGlobal("cancelAnimationFrame", vi.fn())
  document.elementFromPoint = () => null
  root = document.createElement("div")
  document.body.append(root)
  const Child = () => {
    drag = useDragContext()
    return <div />
  }
  dispose = render(
    () => (
      <DragProvider>
        <Child />
      </DragProvider>
    ),
    root
  )
  drag.setOnDrop(handler)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})
function start(id: number, origin?: string) {
  drag.startDrag("instance", [id], point(0), origin)
  drag.updateDrag(point(40))
}
function drop(branch: string) {
  start(1, branch === "unfavorite" ? "favorites" : undefined)
  if (branch === "settle") drag.setDropPreviewRect(new DOMRect(20, 30, 50, 50))
  drag.endDrag()
}
test.each(["settle", "unfavorite", "no-preview"])(
  "%s old cleanup cannot erase an active second drag even if already queued",
  (branch) => {
    drop(branch)
    expect(handler).toHaveBeenCalledTimes(1)
    const obsolete = timeouts.at(-1)!
    start(2)
    obsolete()
    vi.advanceTimersByTime(350)
    frames.forEach((fn) => fn(0))
    expect(drag.isDragging()).toBe(true)
    expect(drag.draggedIds()).toEqual([2])
    expect(drag.dragType()).toBe("instance")
    expect(drag.dropAnimating()).toBeNull()
    drag.endDrag()
    expect(handler).toHaveBeenCalledTimes(2)
    expect(handler.mock.calls[1][1]).toEqual([2])
  }
)
test("queued drop RAF cannot clear justDropped from a newer completed drop", () => {
  drop("settle")
  vi.advanceTimersByTime(300)
  const oldFrames = [...frames]
  frames = []
  drop("settle")
  expect(drag.justDropped()).toBe(true)
  oldFrames.forEach((fn) => fn(0))
  expect(drag.justDropped()).toBe(true)
})
test("reentrant drop handler starting B prevents A from scheduling cleanup or changing B", () => {
  handler.mockImplementationOnce(() => start(2))
  drop("settle")
  expect(drag.isDragging()).toBe(true)
  expect(drag.draggedIds()).toEqual([2])
  expect(drag.dropAnimating()).toBeNull()
  vi.advanceTimersByTime(350)
  expect(drag.draggedIds()).toEqual([2])
  drag.endDrag()
  expect(handler).toHaveBeenCalledTimes(2)
})
test("ordinary ghost settles, fresh drag works, and below-threshold click never dispatches", () => {
  drop("settle")
  expect(drag.dropAnimating()?.type).toBe("settle")
  expect(drag.draggedIds()).toEqual([1])
  vi.advanceTimersByTime(300)
  expect(drag.isDragging()).toBe(false)
  expect(drag.draggedIds()).toEqual([])
  start(3)
  drag.endDrag()
  expect(handler).toHaveBeenCalledTimes(2)
  drag.startDrag("group", [4], point(0))
  drag.updateDrag(point(4))
  drag.endDrag()
  expect(handler).toHaveBeenCalledTimes(2)
})
test.each(["cancel", "escape", "dispose"])(
  "%s revokes queued callbacks and resets residual animation",
  (mode) => {
    drop("unfavorite")
    const obsolete = timeouts.at(-1)!
    if (mode === "dispose") {
      dispose!()
      dispose = undefined
    } else if (mode === "escape") {
      start(2)
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }))
    } else drag.cancelDrag()
    obsolete()
    expect(drag.dropAnimating()).toBeNull()
    expect(drag.isDragging()).toBe(false)
    expect(drag.draggedIds()).toEqual([])
    expect(drag.justDropped()).toBe(false)
  }
)

test("no-preview A delayed clearing cannot erase B before its movement threshold", () => {
  drop("no-preview")
  const obsolete = timeouts.at(-1)!
  drag.startDrag("server", [2], point(0))
  obsolete()
  expect(drag.draggedIds()).toEqual([2])
  drag.updateDrag(point(40))
  drag.endDrag()
  expect(handler).toHaveBeenCalledTimes(2)
})
