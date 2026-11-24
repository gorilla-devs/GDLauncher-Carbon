import { describe, expect, it, vi } from "vitest"
import { backgroundModel, retryVideoOnce, videoAction } from "../background"
import type { MotionState } from "../motion"

const setup = (
  onReload?: (el: unknown) => void,
  motion: MotionState = "run"
) => {
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
  const error = vi.spyOn(console, "error").mockImplementation(() => {})
  const timers: (() => void)[] = []
  const delays: number[] = []
  const cleared: unknown[] = []
  const retry = retryVideoOnce(motion, {
    setTimeout: (cb, ms) => {
      timers.push(cb)
      delays.push(ms)
      return timers.length
    },
    clearTimeout: (id) => cleared.push(id),
    onReload
  })
  const el = (src: string) => ({ src, load: vi.fn() })
  const fire = (e: ReturnType<typeof el>) =>
    retry.onError({ currentTarget: e } as unknown as Event)
  const done = () => {
    warn.mockRestore()
    error.mockRestore()
  }
  return { warn, error, timers, delays, cleared, retry, el, fire, done }
}

describe("retryVideoOnce", () => {
  it("reloads once after a short delay and keeps warning", () => {
    const { warn, error, timers, delays, el, fire, done } = setup()
    try {
      const v = el("http://127.0.0.1:4650/theme/asset?id=a&path=bg.mp4")
      fire(v)
      expect(warn).toHaveBeenCalledWith(
        "Theme background video failed to load: bg.mp4"
      )
      expect(v.load).not.toHaveBeenCalled()
      expect(delays).toEqual([1000])
      timers[0]()
      expect(v.load).toHaveBeenCalledTimes(1)

      // the retry fails too: warn again, never a second reload
      fire(v)
      expect(warn).toHaveBeenCalledTimes(2)
      expect(timers).toHaveLength(1)
      expect(error).not.toHaveBeenCalled()
    } finally {
      done()
    }
  })

  it("a reload under a playing model plays again; pause and off stay still", () => {
    const bg = { src: "a.mp4", video: true, motion: [] } as never
    const actions: string[] = []
    let state: "run" | "pause" | "off" = "run"
    const { timers, el, fire, done } = setup(() => {
      actions.push(videoAction(null, backgroundModel(bg, state)))
    })
    try {
      const v = el("http://x/a.mp4")
      fire(v)
      timers[0]()
      expect(actions).toEqual(["play"])
      state = "pause"
      fire(el("http://x/b.mp4"))
      timers[1]()
      state = "off"
      fire(el("http://x/c.mp4"))
      timers[2]()
      expect(actions).toEqual(["play", "pause", "rewind"])
    } finally {
      done()
    }
  })

  it("a different URL gets its own retry", () => {
    const { timers, el, fire, done } = setup()
    try {
      fire(el("http://x/a.mp4"))
      fire(el("http://x/b.mp4"))
      expect(timers).toHaveLength(2)
    } finally {
      done()
    }
  })

  it("a good load re-arms the URL for a later failure", () => {
    const { timers, el, fire, retry, done } = setup()
    try {
      const v = el("http://x/a.mp4")
      fire(v)
      timers[0]()
      retry.onLoaded()
      fire(v)
      expect(timers).toHaveLength(2)
    } finally {
      done()
    }
  })

  it("a stream that loads and then fails every time stops after three reloads", () => {
    const { timers, el, fire, retry, done } = setup()
    try {
      const v = el("http://x/a.mp4")
      for (let i = 0; i < 6; i++) {
        fire(v)
        timers[i]?.()
        retry.onLoaded()
      }
      expect(timers).toHaveLength(3)
    } finally {
      done()
    }
  })

  it("skips the reload when the source changed meanwhile", () => {
    const onReload = vi.fn()
    const { timers, el, fire, done } = setup(onReload)
    try {
      const v = el("http://x/a.mp4")
      fire(v)
      v.src = "http://x/b.mp4"
      timers[0]()
      expect(v.load).not.toHaveBeenCalled()
      expect(onReload).not.toHaveBeenCalled()
    } finally {
      done()
    }
  })

  it("dispose clears the pending timer", () => {
    const { cleared, el, fire, retry, done } = setup()
    try {
      fire(el("http://x/a.mp4"))
      retry.dispose()
      expect(cleared).toEqual([1])
    } finally {
      done()
    }
  })

  it("no timer runs under pause or off: the retry waits for run", () => {
    const { warn, timers, delays, el, fire, retry, done } = setup(
      undefined,
      "pause"
    )
    try {
      const v = el("http://x/a.mp4")
      fire(v)
      // still warns at once
      expect(warn).toHaveBeenCalledTimes(1)
      expect(timers).toHaveLength(0)
      retry.setMotion("off")
      expect(timers).toHaveLength(0)
      retry.setMotion("run")
      expect(delays).toEqual([1000])
      timers[0]()
      expect(v.load).toHaveBeenCalledTimes(1)
      // paid: back to run again starts nothing
      retry.setMotion("pause")
      retry.setMotion("run")
      expect(timers).toHaveLength(1)
    } finally {
      done()
    }
  })

  it("leaving run drops the timer and keeps the retry owed", () => {
    const { timers, cleared, el, fire, retry, done } = setup()
    try {
      const v = el("http://x/a.mp4")
      fire(v)
      expect(timers).toHaveLength(1)
      retry.setMotion("pause")
      expect(cleared).toEqual([1])
      retry.setMotion("run")
      expect(timers).toHaveLength(2)
      timers[1]()
      expect(v.load).toHaveBeenCalledTimes(1)
    } finally {
      done()
    }
  })

  it("dispose drops a retry owed while paused", () => {
    const { timers, el, fire, retry, done } = setup(undefined, "off")
    try {
      fire(el("http://x/a.mp4"))
      retry.dispose()
      retry.setMotion("run")
      expect(timers).toHaveLength(0)
    } finally {
      done()
    }
  })
})
