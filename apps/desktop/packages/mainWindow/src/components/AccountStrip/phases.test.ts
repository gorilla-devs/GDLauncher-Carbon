import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { createRoot } from "solid-js"
import { createStripPhases } from "./phases"

describe("the account strip's phases", () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  const setup = (closeMs = 300) =>
    createRoot((dispose) => {
      const frames: (() => void)[] = []
      const phases = createStripPhases({
        closeMs: () => closeMs,
        nextFrame: (cb) => frames.push(cb)
      })
      const flushFrame = () => frames.splice(0).forEach((cb) => cb())
      return { phases, flushFrame, dispose }
    })

  it("opening mounts the strip first and shows it on the next frame", () => {
    const { phases, flushFrame, dispose } = setup()
    phases.open()
    expect(phases.mounted()).toBe(true)
    expect(phases.shown()).toBe(false)
    flushFrame()
    expect(phases.shown()).toBe(true)
    dispose()
  })

  it("closing hides it at once and unmounts it after the animation", () => {
    const { phases, flushFrame, dispose } = setup(300)
    phases.open()
    flushFrame()
    phases.close()
    expect(phases.shown()).toBe(false)
    expect(phases.mounted()).toBe(true)
    vi.advanceTimersByTime(299)
    expect(phases.mounted()).toBe(true)
    vi.advanceTimersByTime(1)
    expect(phases.mounted()).toBe(false)
    dispose()
  })

  it("reopening while it closes keeps it mounted and shows it again", () => {
    const { phases, flushFrame, dispose } = setup(300)
    phases.open()
    flushFrame()
    phases.close()
    vi.advanceTimersByTime(150)
    phases.open()
    flushFrame()
    vi.advanceTimersByTime(500)
    expect(phases.mounted()).toBe(true)
    expect(phases.shown()).toBe(true)
    dispose()
  })

  it("closing before the first frame never shows it", () => {
    const { phases, flushFrame, dispose } = setup(300)
    phases.open()
    phases.close()
    flushFrame()
    expect(phases.shown()).toBe(false)
    vi.advanceTimersByTime(300)
    expect(phases.mounted()).toBe(false)
    dispose()
  })

  it("without an animation it unmounts at once", () => {
    const { phases, flushFrame, dispose } = setup(0)
    phases.open()
    flushFrame()
    phases.close()
    expect(phases.mounted()).toBe(false)
    dispose()
  })
})
