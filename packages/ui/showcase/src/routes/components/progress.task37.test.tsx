import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createRoot, onCleanup } from "solid-js"
import { Route } from "./progress"

const disposers: (() => void)[] = []
afterEach(() => {
  disposers.splice(0).forEach((d) => d())
  vi.clearAllTimers()
  vi.useRealTimers()
  vi.restoreAllMocks()
  document.body.textContent = ""
})
it("GPT0183 actual progress route releases its repeating owner after each of two visits", () => {
  vi.useFakeTimers()
  const schedule = globalThis.setInterval
  let ticks = 0
  vi.spyOn(globalThis, "setInterval").mockImplementation(((
    callback: () => void,
    delay: number
  ) =>
    schedule(() => {
      ticks++
      callback()
    }, delay)) as typeof setInterval)
  const counts: number[] = []
  const Page = Route.options.component as () => any
  for (let visit = 0; visit < 2; visit++) {
    const root = document.createElement("div")
    document.body.append(root)
    const dispose = render(() => <Page />, root)
    disposers.push(dispose)
    vi.advanceTimersByTime(100)
    expect(root.textContent).toContain("Progress: 1%")
    dispose()
    counts.push(vi.getTimerCount())
    root.remove()
  }
  expect(counts).toEqual([0, 0])
  const before = ticks
  vi.advanceTimersByTime(1000)
  expect(ticks).toBe(before)
})
it("live progress keeps incrementing and wraps while mounted", () => {
  vi.useFakeTimers()
  const Page = Route.options.component as () => any
  const root = document.createElement("div")
  document.body.append(root)
  disposers.push(render(() => <Page />, root))
  vi.advanceTimersByTime(10000)
  expect(root.textContent).toContain("Progress: 100%")
  vi.advanceTimersByTime(100)
  expect(root.textContent).toContain("Progress: 1%")
})
it("explicit same-owner interval cleanup leaves zero timers", () => {
  vi.useFakeTimers()
  const dispose = createRoot((dispose) => {
    const timer = setInterval(() => {}, 100)
    onCleanup(() => clearInterval(timer))
    return dispose
  })
  expect(vi.getTimerCount()).toBe(1)
  dispose()
  expect(vi.getTimerCount()).toBe(0)
})
