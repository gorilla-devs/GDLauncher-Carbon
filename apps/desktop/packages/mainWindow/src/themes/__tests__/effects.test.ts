import { describe, it, expect, vi } from "vitest"
import { createEffectsLoop, type Effect } from "../effects/loop"
import {
  createSnow,
  flakeCount,
  resolveSnowOptions,
  SNOW_DEFAULTS,
  type Flake
} from "../effects/snow"
import {
  blurredDisc,
  createLights,
  GLOW_RADIUS,
  layoutLights,
  LIGHTS_DEFAULTS,
  LIGHTS_HEIGHT,
  lightsHeight
} from "../effects/lights"

function seeded(seed: number): () => number {
  let a = seed
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

// A 2D context that records the name of every method called on it.
function fakeCtx(): {
  ctx: CanvasRenderingContext2D
  calls: string[]
  store: Record<string, unknown>
} {
  const calls: string[] = []
  const store: Record<string, unknown> = {
    canvas: { width: 0, height: 0, style: {} }
  }
  const ctx = new Proxy(store, {
    get(target, key: string) {
      if (key in target) return target[key]
      return (..._args: unknown[]) => {
        calls.push(key)
        return key.startsWith("create") ? { addColorStop() {} } : undefined
      }
    },
    set(target, key: string, value) {
      target[key] = value
      return true
    }
  }) as unknown as CanvasRenderingContext2D
  return { ctx, calls, store }
}

const fakeCanvas = () => {
  const { ctx } = fakeCtx()
  return {
    width: 0,
    height: 0,
    getContext: () => ctx
  } as unknown as HTMLCanvasElement
}

// A fake display and timer queue on one clock.
function fakeRaf() {
  let next = 1
  let now = 0
  let requested = 0
  const delays: number[] = []
  const pending = new Map<number, (t: number) => void>()
  const timers = new Map<number, { at: number; cb: () => void }>()
  return {
    raf(cb: (t: number) => void) {
      requested++
      pending.set(next, cb)
      return next++
    },
    caf(id: number) {
      pending.delete(id)
    },
    now: () => now,
    delays,
    setTimeout(cb: () => void, ms: number) {
      delays.push(ms)
      timers.set(next, { at: now + ms, cb })
      return next++
    },
    clearTimeout(id: number) {
      timers.delete(id)
    },
    pending: () => pending.size,
    timers: () => timers.size,
    requested: () => requested,
    // A display refresh at time t: the timers due by then fire first, in
    // order, then the pending frame callback runs.
    step(t: number) {
      for (;;) {
        const due = [...timers.entries()]
          .filter(([, x]) => x.at <= t)
          .sort((a, b) => a[1].at - b[1].at)[0]
        if (!due) break
        timers.delete(due[0])
        now = due[1].at
        due[1].cb()
      }
      now = t
      const entries = [...pending.entries()]
      pending.clear()
      for (const [, cb] of entries) cb(t)
    }
  }
}

function recorder(fps: number) {
  const frames: { t: number; dt: number }[] = []
  const effect: Effect = {
    fps,
    resize() {},
    frame(t, dt) {
      frames.push({ t, dt })
    }
  }
  return { effect, frames }
}

describe("effects loop", () => {
  it("caps each effect at its own frame rate", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    const a = recorder(30)
    const b = recorder(20)
    loop.add(a.effect)
    loop.add(b.effect)
    loop.setRunning(true)
    for (let t = 0; t <= 1000; t += 16.7) r.step(t)
    expect(Math.abs(a.frames.length - 30)).toBeLessThanOrEqual(1)
    expect(Math.abs(b.frames.length - 20)).toBeLessThanOrEqual(1)
  })

  it("stops scheduling while paused", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    loop.add(recorder(30).effect)
    loop.setRunning(true)
    expect(loop.isScheduled()).toBe(true)
    loop.setRunning(false)
    expect(loop.isScheduled()).toBe(false)
    expect(r.pending()).toBe(0)
    loop.setRunning(true)
    expect(loop.isScheduled()).toBe(true)
    expect(r.pending()).toBe(1)
  })

  it("the timer waits from now, not from the frame time", () => {
    const r = fakeRaf()
    // the frames took 12 ms: the clock reads 1012 when the raf says 1000
    const loop = createEffectsLoop({ ...r, now: () => 1012 })
    loop.add(recorder(30).effect)
    loop.setRunning(true)
    r.step(1000)
    expect(r.delays).toHaveLength(1)
    // due at 1000 + 33.33 - 1 (slack); 12 ms are already gone; 2 ms of lead
    expect(
      Math.abs(r.delays[0] - (1000 + 1000 / 30 - 1 - 1012 - 2))
    ).toBeLessThan(0.01)
  })

  it("a frame that ran past its slot requests the next one at once", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop({ ...r, now: () => 1040 })
    loop.add(recorder(30).effect)
    loop.setRunning(true)
    r.step(1000)
    expect(r.delays).toHaveLength(0)
    expect(r.pending()).toBe(1)
  })

  it("waits for the next due frame with a timer, and pausing cancels it", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    loop.add(recorder(30).effect)
    loop.setRunning(true)
    r.step(0)
    expect(r.pending()).toBe(0)
    expect(r.timers()).toBe(1)
    expect(loop.isScheduled()).toBe(true)
    loop.setRunning(false)
    expect(r.timers()).toBe(0)
    expect(r.pending()).toBe(0)
    expect(loop.isScheduled()).toBe(false)
  })

  it("requests a frame about once per due frame at 240 Hz, not on every refresh", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    const a = recorder(30)
    const b = recorder(20)
    loop.add(a.effect)
    loop.add(b.effect)
    loop.setRunning(true)
    const period = 1000 / 240
    let refreshes = 0
    for (let k = 0; k * period <= 1000; k++, refreshes++) r.step(k * period)
    expect(Math.abs(a.frames.length - 30)).toBeLessThanOrEqual(1)
    expect(Math.abs(b.frames.length - 20)).toBeLessThanOrEqual(1)
    const drawn = new Set([...a.frames, ...b.frames].map((f) => f.t)).size
    expect(r.requested()).toBeLessThanOrEqual(drawn + 1)
    expect(r.requested()).toBeLessThan(refreshes / 4)
  })

  it("an effect added while waiting draws on the next refresh", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    loop.add(recorder(30).effect)
    loop.setRunning(true)
    r.step(0)
    const b = recorder(20)
    loop.add(b.effect)
    expect(r.pending()).toBe(1)
    expect(r.timers()).toBe(0)
    r.step(4)
    expect(b.frames).toEqual([{ t: 4, dt: 0 }])
  })

  it("schedules nothing while it has no effects", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    const remove = loop.add(recorder(30).effect)
    loop.setRunning(true)
    r.step(0)
    remove()
    r.step(100)
    expect(loop.isScheduled()).toBe(false)
    expect(r.pending()).toBe(0)
    expect(r.timers()).toBe(0)
  })

  it("the first frame after a pause does not jump", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    const a = recorder(30)
    loop.add(a.effect)
    loop.setRunning(true)
    r.step(0)
    r.step(34)
    loop.setRunning(false)
    loop.setRunning(true)
    r.step(34 + 3_600_000)
    r.step(34 + 3_600_000 + 34)
    r.step(34 + 3_600_000 + 68)
    const resumed = a.frames.slice(2)
    expect(resumed[0].dt).toBe(0)
    for (const f of resumed.slice(1)) {
      expect(f.dt).toBeGreaterThan(0)
      expect(f.dt).toBeLessThanOrEqual(0.05)
    }
  })

  it("caps dt at 50 ms after a long gap while running", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    const a = recorder(30)
    loop.add(a.effect)
    loop.setRunning(true)
    r.step(0)
    r.step(5000)
    expect(a.frames[1].dt).toBe(0.05)
  })

  it("a removed effect gets no more frames", () => {
    const r = fakeRaf()
    const loop = createEffectsLoop(r)
    const a = recorder(30)
    const remove = loop.add(a.effect)
    loop.setRunning(true)
    r.step(0)
    remove()
    r.step(100)
    expect(a.frames.length).toBe(1)
  })
})

describe("snow", () => {
  const sprite = () => ({}) as CanvasImageSource

  function make(opts: Parameters<typeof createSnow>[0], seed = 7) {
    const { ctx, calls, store } = fakeCtx()
    const snow = createSnow(opts, ctx, { random: seeded(seed), sprite })
    return { snow, calls, store }
  }

  it("scales flakes with the area and caps them", () => {
    expect(flakeCount(21, 1160, 926)).toBe(226)
    expect(flakeCount(60, 4000, 3000)).toBe(600)
    const { snow } = make({ density: 21 })
    snow.resize(1160, 926)
    const flakes = snow.flakes()
    expect(flakes.length).toBe(226)
    const share = (l: number) =>
      (flakes.filter((f) => f.layer === l).length / flakes.length) * 100
    expect(Math.abs(share(0) - 55)).toBeLessThanOrEqual(8)
    expect(Math.abs(share(1) - 33)).toBeLessThanOrEqual(8)
    expect(Math.abs(share(2) - 12)).toBeLessThanOrEqual(8)
    for (const f of flakes) {
      expect(f.r).toBeGreaterThanOrEqual(0.6)
      expect(f.r).toBeLessThanOrEqual(4.2)
    }
  })

  it("honours the layer count", () => {
    const one = make({ layers: 1 }).snow
    one.resize(1000, 1000)
    expect(new Set(one.flakes().map((f) => f.layer))).toEqual(new Set([1]))
    const two = make({ layers: 2 }).snow
    two.resize(1000, 1000)
    expect(new Set(two.flakes().map((f) => f.layer)).has(2)).toBe(false)
  })

  it("honours size and speed", () => {
    const sized = make({ minSize: 1, maxSize: 2 }).snow
    sized.resize(1160, 926)
    for (const f of sized.flakes()) {
      expect(f.r).toBeGreaterThanOrEqual(1)
      expect(f.r).toBeLessThanOrEqual(2)
    }
    const mean = (fl: readonly Flake[]) =>
      fl.reduce((s, f) => s + f.vy, 0) / fl.length
    const base = make({}).snow
    base.resize(1160, 926)
    const fast = make({ speed: 2 }).snow
    fast.resize(1160, 926)
    const ratio = mean(fast.flakes()) / mean(base.flakes())
    expect(ratio).toBeGreaterThan(1.8)
    expect(ratio).toBeLessThan(2.2)
  })

  it("wind moves surviving flakes by about 30 px per second", () => {
    const calm = make({ wind: 0 }).snow
    const windy = make({ wind: 30 }).snow
    calm.resize(1160, 926)
    windy.resize(1160, 926)
    const before = {
      calm: calm.flakes().map((f) => ({ x: f.x, y: f.y })),
      windy: windy.flakes().map((f) => ({ x: f.x, y: f.y }))
    }
    // Both runs see identical frame times, so only the wind differs.
    const dt = 1 / 30
    for (let i = 1; i <= 30; i++) {
      calm.frame(i * 33.3, dt)
      windy.frame(i * 33.3, dt)
    }
    const diffs: number[] = []
    calm.flakes().forEach((f, i) => {
      const w = windy.flakes()[i]
      const respawned =
        f.y < before.calm[i].y ||
        w.y < before.windy[i].y ||
        Math.abs(w.x - f.x) > 100
      if (respawned) return
      diffs.push(w.x - before.windy[i].x - (f.x - before.calm[i].x))
    })
    expect(diffs.length).toBeGreaterThan(100)
    const mean = diffs.reduce((s, d) => s + d, 0) / diffs.length
    expect(Math.abs(mean - 30)).toBeLessThanOrEqual(10)
  })

  it("respawns a flake that leaves the bottom at the top", () => {
    const { snow } = make({})
    snow.resize(200, 100)
    for (let i = 1; i <= 400; i++) snow.frame(i * 33, 0.05)
    for (const f of snow.flakes()) {
      expect(f.y).toBeGreaterThanOrEqual(-6)
      expect(f.y).toBeLessThanOrEqual(106)
    }
  })

  it("draws sprites, never shadowBlur", () => {
    const { snow, calls, store } = make({})
    snow.resize(200, 100)
    const before = calls.filter((c) => c === "drawImage").length
    snow.frame(33, 1 / 30)
    expect(calls.filter((c) => c === "drawImage").length - before).toBe(
      snow.flakes().length
    )
    expect("shadowBlur" in store).toBe(false)
  })

  describe("size range", () => {
    it("a minSize above the default maxSize raises maxSize", () => {
      const o = resolveSnowOptions({ minSize: 6 })
      expect(o.minSize).toBe(6)
      expect(o.maxSize).toBe(6)
    })
    it("a maxSize below the default minSize lowers minSize", () => {
      const o = resolveSnowOptions({ maxSize: 0.3 })
      expect(o.maxSize).toBe(0.3)
      expect(o.minSize).toBe(0.3)
    })
    it("ignores unset fields", () => {
      expect(resolveSnowOptions({ density: undefined })).toEqual(SNOW_DEFAULTS)
    })
  })
})

describe("string lights", () => {
  const spacing = 161.7
  const opts = { spacing, droop: 17, colors: LIGHTS_DEFAULTS.colors }

  it("lays lights along the content width", () => {
    const { hooks, bulbs } = layoutLights(1160, opts, seeded(3))
    expect(hooks.length).toBe(8)
    expect(hooks[0]).toBe(14)
    expect(hooks[hooks.length - 1]).toBeCloseTo(1146, 6)
    expect(bulbs.length).toBe(21)
    bulbs.forEach((b, i) => {
      expect(b.color).toBe(LIGHTS_DEFAULTS.colors[i % 8])
      expect(b.y).toBeGreaterThanOrEqual(8)
      expect(b.y).toBeLessThanOrEqual(48)
      expect(b.period).toBeGreaterThanOrEqual(2.8)
      expect(b.period).toBeLessThanOrEqual(5.8)
    })
  })

  it("fits whole spans at other widths", () => {
    const { hooks } = layoutLights(500, opts, seeded(3))
    expect(hooks[0]).toBe(14)
    expect(hooks[hooks.length - 1]).toBeCloseTo(486, 6)
  })

  function make() {
    const { ctx, calls, store } = fakeCtx()
    const lights = createLights({}, ctx, {
      random: seeded(5),
      createCanvas: fakeCanvas
    })
    return { lights, calls, store }
  }

  it("draws once under reduced motion, with no raf", () => {
    const raf = vi.fn(() => 1)
    vi.stubGlobal("requestAnimationFrame", raf)
    try {
      const { lights, calls, store } = make()
      lights.resize(1160, 72)
      const before = calls.length
      lights.drawStatic()
      expect(calls.length).toBeGreaterThan(before)
      expect(calls).toContain("drawImage")
      expect(raf).not.toHaveBeenCalled()
      expect("shadowBlur" in store).toBe(false)
    } finally {
      vi.unstubAllGlobals()
    }
  })

  it("lays out identically from the same seed", () => {
    const a = layoutLights(1160, opts, seeded(11))
    const b = layoutLights(1160, opts, seeded(11))
    expect(a).toEqual(b)
    expect(layoutLights(1160, opts, seeded(12))).not.toEqual(a)
  })

  it("a seeded light string replays its layout on every resize", () => {
    const random = vi.fn(Math.random)
    const { ctx } = fakeCtx()
    const lights = createLights({}, ctx, {
      random,
      seed: 99,
      createCanvas: fakeCanvas
    })
    lights.resize(1160, 72)
    lights.resize(900, 72)
    lights.resize(1160, 72)
    expect(random).not.toHaveBeenCalled()
  })

  describe("canvas height", () => {
    it("is 72 for the defaults", () => {
      expect(lightsHeight({})).toBe(LIGHTS_HEIGHT)
    })
    it("holds the lowest glow at droop 40 and size 2", () => {
      const h = lightsHeight({ droop: 40, size: 2 })
      // wire 8 + droop 40 + 2 jitter, then the glow, centred 12 below the
      // socket, reaching GLOW_RADIUS further, all scaled by size
      expect(h).toBeGreaterThanOrEqual(8 + 40 + 2 + (12 + GLOW_RADIUS) * 2)
      expect(h).toBeGreaterThan(LIGHTS_HEIGHT)
    })
    it("holds the default glow without growing", () => {
      // the lowest default bulb: wire 8, droop 17 + 2 jitter, half way
      const bulbY = 8 + (17 + 2)
      expect(bulbY + (12 + GLOW_RADIUS) * 1.15).toBeLessThanOrEqual(
        lightsHeight({})
      )
      expect(lightsHeight({})).toBe(LIGHTS_HEIGHT)
    })
  })

  describe("bulb glow", () => {
    // The mock's glow: a disc of radius 10 blurred with sigma 4.5.
    it("is a blurred disc: bright in the middle, about half at its edge", () => {
      expect(blurredDisc(0, 10, 4.5)).toBeCloseTo(1 - Math.exp(-100 / 40.5), 2)
      expect(blurredDisc(10, 10, 4.5)).toBeGreaterThan(0.4)
      expect(blurredDisc(10, 10, 4.5)).toBeLessThan(0.55)
      expect(blurredDisc(5, 10, 4.5)).toBeGreaterThan(blurredDisc(10, 10, 4.5))
    })

    it("has faded out by its radius, which reaches well past the bulb", () => {
      expect(GLOW_RADIUS).toBeGreaterThanOrEqual(18)
      expect(blurredDisc(GLOW_RADIUS, 10, 4.5)).toBeLessThan(0.03)
    })

    it("draws every bulb's glow at the full radius", () => {
      const sizes: number[][] = []
      const { ctx } = fakeCtx()
      const recording = new Proxy(ctx as unknown as Record<string, unknown>, {
        get(target, key: string) {
          if (key === "drawImage") {
            return (...args: number[]) => {
              sizes.push(args.slice(3, 5))
            }
          }
          return target[key]
        }
      }) as unknown as CanvasRenderingContext2D
      const lights = createLights({}, recording, {
        random: seeded(5),
        createCanvas: fakeCanvas
      })
      lights.resize(1160, 72)
      sizes.length = 0
      lights.drawStatic()
      const glows = sizes.filter(
        ([w, h]) => w === GLOW_RADIUS * 2 && h === GLOW_RADIUS * 2
      )
      expect(glows).toHaveLength(21)
    })
  })

  it("redraws on frame", () => {
    const { lights, calls } = make()
    lights.resize(1160, 72)
    const before = calls.length
    lights.frame(1000, 0.05)
    expect(calls.length).toBeGreaterThan(before)
  })
})
