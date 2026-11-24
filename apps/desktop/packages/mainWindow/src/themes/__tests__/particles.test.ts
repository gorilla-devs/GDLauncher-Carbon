import { describe, it, expect, vi } from "vitest"
import {
  createParticles,
  MAX_PARTICLES,
  particleCount,
  PARTICLES_DEFAULTS,
  PARTICLES_FPS,
  resolveSizeRange,
  spriteRasterSize,
  type ParticlePreset
} from "../effects/particles"
import {
  createSpriteCache,
  drawSprite,
  rasterizeSprite,
  spriteKind
} from "../effects/sprites"

function seeded(seed: number): () => number {
  let a = seed
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

interface Call {
  name: string
  args: unknown[]
}

// A 2D context that records every method call with its arguments.
function fakeCtx() {
  const calls: Call[] = []
  const store: Record<string, unknown> = {
    canvas: { width: 0, height: 0, style: {} }
  }
  const ctx = new Proxy(store, {
    get(target, key: string) {
      if (key in target) return target[key]
      return (...args: unknown[]) => {
        calls.push({ name: key, args })
      }
    },
    set(target, key: string, value) {
      target[key] = value
      return true
    }
  }) as unknown as CanvasRenderingContext2D
  return { ctx, calls, store }
}

const spriteA = { width: 20, height: 10 } as unknown as CanvasImageSource
const spriteB = { width: 10, height: 10 } as unknown as CanvasImageSource

const W = 1160
const H = 926

type Opts = Parameters<typeof createParticles>[0]

function make(
  opts: Partial<Opts> & { preset: ParticlePreset },
  sprites: (CanvasImageSource | null)[] = [spriteA, spriteB],
  seed = 7
) {
  const { ctx, calls, store } = fakeCtx()
  const p = createParticles({ spriteCount: sprites.length, ...opts }, ctx, {
    random: seeded(seed),
    sprite: (i) => sprites[i] ?? null
  })
  p.resize(W, H)
  return { p, calls, store }
}

// The mean per-second displacement of the particles that didn't respawn.
function meanVelocity(
  preset: ParticlePreset,
  axis: "x" | "y",
  extra: Partial<Opts> = {}
): number {
  const { p } = make({ preset, density: 12, ...extra })
  const frames = 60
  const dt = 1 / 30
  const before = p.particles().map((q) => ({ x: q.x, y: q.y }))
  const jumped = new Set<number>()
  let prev = before
  for (let i = 1; i <= frames; i++) {
    p.frame((i * 1000) / 30, dt)
    p.particles().forEach((q, k) => {
      // a respawn moves a particle by far more than a frame's travel
      if (Math.abs(q.x - prev[k].x) > 60 || Math.abs(q.y - prev[k].y) > 60) {
        jumped.add(k)
      }
    })
    prev = p.particles().map((q) => ({ x: q.x, y: q.y }))
  }
  let sum = 0
  let n = 0
  p.particles().forEach((q, k) => {
    if (jumped.has(k)) return
    sum += (q[axis] - before[k][axis]) / (frames * dt)
    n++
  })
  expect(n).toBeGreaterThan(10)
  return sum / n
}

describe("particles", () => {
  it("scales with the area and is capped", () => {
    expect(particleCount(6, 1160, 926)).toBe(64)
    expect(particleCount(30, 4000, 3000)).toBe(150)
    expect(MAX_PARTICLES).toBe(150)
    expect(PARTICLES_FPS).toBe(30)
    expect(particleCount(6, 0, 0)).toBe(0)
  })

  it("each preset moves its own way", () => {
    const fall = meanVelocity("fall", "y")
    expect(fall).toBeGreaterThan(30)
    expect(fall).toBeLessThan(60)
    const float = meanVelocity("float", "y")
    expect(float).toBeGreaterThan(-30)
    expect(float).toBeLessThan(-14)
    const drift = meanVelocity("drift", "x")
    expect(drift).toBeGreaterThan(28)
    expect(drift).toBeLessThan(56)
    const fast = meanVelocity("fall", "y", { speed: 2 })
    expect(fast / fall).toBeGreaterThan(0.9 * 2)
    expect(fast / fall).toBeLessThan(1.1 * 2)
  })

  it("sizes, opacity and rotation follow the options", () => {
    const { p } = make({
      preset: "fall",
      minSize: 10,
      maxSize: 12,
      rotate: false,
      opacity: 0.5
    })
    expect(p.particles().length).toBeGreaterThan(0)
    for (const q of p.particles()) {
      expect(q.size).toBeGreaterThanOrEqual(10)
      expect(q.size).toBeLessThanOrEqual(12)
      expect(q.spin).toBe(0)
      expect(q.alpha).toBeLessThanOrEqual(0.5)
    }
    p.frame(33, 1 / 30)
    for (const q of p.particles()) {
      expect(q.angle).toBe(0)
      expect(q.alpha).toBeLessThanOrEqual(0.5)
    }
  })

  it("null options fall back to the defaults", () => {
    const { p } = make({
      preset: "fall",
      density: null,
      speed: null,
      minSize: null,
      maxSize: null,
      rotate: null,
      opacity: null
    })
    expect(p.particles()).toHaveLength(
      particleCount(PARTICLES_DEFAULTS.density, W, H)
    )
    for (const q of p.particles()) {
      expect(q.size).toBeGreaterThanOrEqual(PARTICLES_DEFAULTS.minSize)
      expect(q.size).toBeLessThanOrEqual(PARTICLES_DEFAULTS.maxSize)
      expect(q.alpha).toBeCloseTo(PARTICLES_DEFAULTS.opacity)
    }
  })

  it("particles respawn and keep their count", () => {
    for (const preset of ["fall", "float", "drift"] as const) {
      const { p } = make({ preset })
      const count = p.particles().length
      for (let i = 1; i <= 600; i++) p.frame((i * 1000) / 30, 1 / 30)
      expect(p.particles()).toHaveLength(count)
      for (const q of p.particles()) {
        expect(q.x).toBeGreaterThanOrEqual(-q.size)
        expect(q.x).toBeLessThanOrEqual(W + q.size)
        expect(q.y).toBeGreaterThanOrEqual(-q.size)
        expect(q.y).toBeLessThanOrEqual(H + q.size)
      }
    }
  })

  it("float fades in and out over its run", () => {
    const { p } = make({ preset: "float", opacity: 1 })
    for (const q of p.particles()) {
      const prog = (H + q.size - q.y) / (H + 2 * q.size)
      expect(q.alpha).toBeCloseTo(Math.sin(Math.PI * prog), 1)
    }
  })

  it("a missing sprite is skipped", () => {
    const { p, calls } = make({ preset: "fall", density: 12 }, [spriteA, null])
    p.frame(33, 1 / 30)
    const draws = calls.filter((c) => c.name === "drawImage")
    expect(draws.length).toBeGreaterThan(0)
    expect(draws.every((c) => c.args[0] === spriteA)).toBe(true)
    expect(draws.some((c) => c.args[0] === null)).toBe(false)
  })

  it("draws with the sprite's aspect, rotated, and resets the transform", () => {
    const { p, calls } = make({ preset: "fall" }, [spriteA])
    calls.length = 0
    p.frame(33, 1 / 30)
    const draw = calls.find((c) => c.name === "drawImage")!
    const [, dx, dy, dw, dh] = draw.args as number[]
    expect(dw).toBeGreaterThan(0)
    expect(dh).toBeCloseTo(dw / 2) // spriteA is 2:1
    expect(dx).toBeCloseTo(-dw / 2)
    expect(dy).toBeCloseTo(-dh / 2)
    const names = calls.map((c) => c.name)
    expect(names).toContain("rotate")
    expect(calls.at(-1)).toEqual({
      name: "setTransform",
      args: [1, 0, 0, 1, 0, 0]
    })
  })

  it("resize rebuilds the particles for the new area", () => {
    const { p, store } = make({ preset: "fall" })
    const before = p.particles().length
    p.resize(W * 2, H)
    expect(p.particles().length).toBeGreaterThan(before)
    expect((store.canvas as { width: number }).width).toBe(W * 2)
  })
})

describe("sprite sizing", () => {
  it("a lone minSize raises the range and the raster width", () => {
    expect(resolveSizeRange({ minSize: 60 })).toEqual({ min: 60, max: 60 })
    expect(resolveSizeRange({ maxSize: 8 })).toEqual({ min: 8, max: 8 })
    expect(resolveSizeRange({})).toEqual({ min: 14, max: 28 })
    expect(spriteRasterSize({ minSize: 60 }, 100, 100)).toEqual({
      width: 60,
      height: 60
    })
    expect(spriteRasterSize({ maxSize: 27.2 }, 100, 100).width).toBe(28)
  })

  it("the raster is as wide as the largest size, height from the aspect", () => {
    expect(spriteRasterSize({ maxSize: 40 }, 100, 200)).toEqual({
      width: 40,
      height: 80
    })
    expect(spriteRasterSize({ maxSize: 40 }, 200, 100)).toEqual({
      width: 40,
      height: 20
    })
    // a 4000 px source is rasterised at the drawn size, not kept
    expect(spriteRasterSize({ maxSize: 28 }, 4000, 4000)).toEqual({
      width: 28,
      height: 28
    })
    expect(spriteRasterSize({ maxSize: 28 }, 4000, 1)).toEqual({
      width: 28,
      height: 1
    })
    expect(spriteRasterSize({}, 0, 0)).toEqual({ width: 28, height: 28 })
  })

  it("the raster height is bounded", () => {
    expect(spriteRasterSize({ maxSize: 96 }, 1, 4000)).toEqual({
      width: 96,
      height: 384
    })
    expect(spriteRasterSize({ maxSize: 28 }, 1, 4000).height).toBe(112)
    expect(
      spriteRasterSize({ maxSize: 96 }, 1, 400000).height
    ).toBeLessThanOrEqual(512)
  })

  it("every source, an SVG included, is drawn through a canvas", () => {
    const canvases: {
      width: number
      height: number
      draws: unknown[][]
      quality?: string
    }[] = []
    const make = () => {
      const c = {
        width: 0,
        height: 0,
        draws: [] as unknown[][],
        quality: undefined as string | undefined,
        getContext() {
          return {
            set imageSmoothingQuality(v: string) {
              c.quality = v
            },
            drawImage: (...a: unknown[]) => c.draws.push(a)
          }
        }
      }
      canvases.push(c)
      return c as unknown as HTMLCanvasElement
    }
    const bitmap = vi.fn()
    vi.stubGlobal("createImageBitmap", bitmap)
    try {
      const svg = { tag: "svg" } as unknown as CanvasImageSource
      const out = drawSprite(svg, 40, 40, 28, 28, make)
      expect(bitmap).not.toHaveBeenCalled()
      expect(out).toBe(canvases.at(-1))
      expect(canvases.at(-1)!.draws[0]).toEqual([svg, 0, 0, 28, 28])
      expect(canvases.at(-1)!.quality).toBe("high")
    } finally {
      vi.unstubAllGlobals()
    }
  })

  it("a large source is halved in steps before the final draw", () => {
    const sizes: number[][] = []
    const make = () =>
      ({
        width: 0,
        height: 0,
        getContext() {
          return {
            drawImage(
              _s: unknown,
              _x: number,
              _y: number,
              w: number,
              h: number
            ) {
              sizes.push([w, h])
            }
          }
        }
      }) as unknown as HTMLCanvasElement
    drawSprite({} as CanvasImageSource, 4000, 4000, 28, 28, make)
    // 2000 ... 63, 32, then the 28 px target
    expect(sizes.map((s) => s[0])).toEqual([
      2000, 1000, 500, 250, 125, 63, 32, 28
    ])
  })

  // Records every canvas drawSprite makes: its size and what was drawn on it.
  const canvasLog = () => {
    const made: { width: number; height: number; draws: unknown[][] }[] = []
    const make = () => {
      const c = {
        width: 0,
        height: 0,
        draws: [] as unknown[][],
        getContext() {
          return { drawImage: (...a: unknown[]) => c.draws.push(a) }
        }
      }
      made.push(c)
      return c as unknown as HTMLCanvasElement
    }
    return { made, make }
  }

  it("a raster source's first step is at most 2048 px on its long side", () => {
    const { made, make } = canvasLog()
    drawSprite({} as CanvasImageSource, 20000, 10000, 28, 14, make)
    expect(made.map((c) => [c.width, c.height])).toEqual([
      [2048, 1024],
      [1024, 512],
      [512, 256],
      [256, 128],
      [128, 64],
      [64, 32],
      [32, 16],
      [28, 14]
    ])
    // a thin strip keeps at least a pixel
    const strip = canvasLog()
    drawSprite({} as CanvasImageSource, 100000, 10, 96, 1, strip.make)
    expect(strip.made[0].width).toBe(2048)
    expect(strip.made[0].height).toBe(1)
    for (const c of strip.made) {
      expect(Math.max(c.width, c.height)).toBeLessThanOrEqual(2048)
    }
  })

  it("an SVG is drawn straight at the raster size, however large it says it is", () => {
    const { made, make } = canvasLog()
    const svg = { tag: "svg" } as unknown as CanvasImageSource
    const out = drawSprite(svg, 100000, 100000, 28, 28, make, "vector")
    expect(made.map((c) => [c.width, c.height])).toEqual([[28, 28]])
    expect(made[0].draws).toEqual([[svg, 0, 0, 28, 28]])
    expect(out).toBe(made[0])
  })

  it("tells an SVG by its file name, in a core URL or a bundled one", () => {
    const core = (p: string) =>
      `http://127.0.0.1:4650/theme/asset?id=autumn&path=${encodeURIComponent(p)}&token=secret`
    expect(spriteKind(core("assets/leaf.svg"))).toBe("vector")
    expect(spriteKind(core("assets/LEAF.SVG"))).toBe("vector")
    expect(spriteKind("file:///opt/app/assets/leaf-Bx12.svg")).toBe("vector")
    expect(spriteKind("/assets/leaf-Bx12.svg")).toBe("vector")
    expect(spriteKind(core("assets/leaf.png"))).toBe("raster")
    expect(spriteKind(core("assets/leaf.svg.webp"))).toBe("raster")
    expect(spriteKind("file:///opt/app/assets/leaf.gif")).toBe("raster")
  })

  it("rasterises an SVG that declares a huge size in one draw", async () => {
    const { made, make } = canvasLog()
    class FakeImage {
      src = ""
      naturalWidth = 100000
      naturalHeight = 50000
      decode() {
        return Promise.resolve()
      }
    }
    vi.stubGlobal("Image", FakeImage)
    vi.stubGlobal("document", { createElement: make })
    try {
      const url =
        "http://127.0.0.1:4650/theme/asset?id=a&path=assets%2Fhuge.svg&token=t"
      const out = await rasterizeSprite(url, { maxSize: 28 })
      expect(made.map((c) => [c.width, c.height])).toEqual([[28, 14]])
      expect(out).toBe(made[0])
      // the same size as a PNG is stepped down, from at most 2048 px
      made.length = 0
      await rasterizeSprite(url.replace("huge.svg", "huge.png"), {
        maxSize: 28
      })
      expect(made[0].width).toBe(2048)
      expect(made.at(-1)!.width).toBe(28)
    } finally {
      vi.unstubAllGlobals()
    }
  })

  it("tall sprites respawn by their drawn height", () => {
    const tall = { width: 10, height: 40 } as unknown as CanvasImageSource
    const { p } = make({ preset: "fall", minSize: 20, maxSize: 20 }, [tall])
    for (let i = 1; i <= 600; i++) p.frame((i * 1000) / 30, 1 / 30)
    for (const q of p.particles()) {
      // reach is size * aspect = 80
      expect(q.y).toBeGreaterThanOrEqual(-80)
      expect(q.y).toBeLessThanOrEqual(H + 80)
    }
    // a respawned particle starts fully above the top edge
    const above = p.particles().filter((q) => q.y < 0)
    expect(above.length).toBeGreaterThan(0)
  })

  it("reads each sprite's aspect once", () => {
    let reads = 0
    const img = {
      get width() {
        reads++
        return 10
      },
      height: 10
    } as unknown as CanvasImageSource
    const { p } = make({ preset: "fall", density: 12 }, [img])
    for (let i = 1; i <= 10; i++) p.frame((i * 1000) / 30, 1 / 30)
    expect(reads).toBeLessThanOrEqual(3)
  })
})

describe("sprite cache", () => {
  it("loads each image once", async () => {
    const image = {} as CanvasImageSource
    const load = vi.fn(() => Promise.resolve(image))
    const cache = createSpriteCache(load)
    expect(cache.get("a.png")).toBeNull()
    cache.get("a.png")
    await cache.ready("a.png")
    await cache.ready("a.png")
    expect(cache.get("a.png")).toBe(image)
    expect(load).toHaveBeenCalledTimes(1)
    await cache.ready("b.png")
    expect(load).toHaveBeenCalledTimes(2)
  })

  it("a rejected load stays null and warns once", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    try {
      const load = vi.fn(() => Promise.reject(new Error("decode")))
      const cache = createSpriteCache(load)
      cache.get("bad.svg")
      await cache.ready("bad.svg")
      await cache.ready("bad.svg")
      expect(cache.get("bad.svg")).toBeNull()
      expect(load).toHaveBeenCalledTimes(1)
      expect(warn).toHaveBeenCalledTimes(1)
      expect(String(warn.mock.calls[0][0])).toContain("bad.svg")
    } finally {
      warn.mockRestore()
    }
  })

  it("a null load also warns once", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    try {
      const cache = createSpriteCache(() => Promise.resolve(null))
      await cache.ready("gone.png")
      expect(cache.get("gone.png")).toBeNull()
      expect(warn).toHaveBeenCalledTimes(1)
    } finally {
      warn.mockRestore()
    }
  })
})
