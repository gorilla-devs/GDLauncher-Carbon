import { rgba } from "./color"
import type { Effect } from "./loop"

export const SNOW_DEFAULTS = {
  density: 20,
  speed: 1,
  minSize: 0.6,
  maxSize: 4.2,
  wind: 0,
  color: "255 255 255",
  layers: 3
} as const

export const MAX_FLAKES = 600
export const SNOW_FPS = 30

export type SnowOptions = {
  -readonly [K in keyof typeof SNOW_DEFAULTS]: (typeof SNOW_DEFAULTS)[K] extends string
    ? string
    : number
}

export function flakeCount(
  density: number,
  width: number,
  height: number
): number {
  return Math.min(
    MAX_FLAKES,
    Math.max(0, Math.round((density * width * height) / 100_000))
  )
}

export interface Flake {
  x: number
  y: number
  r: number
  vy: number
  alpha: number
  phase: number
  freq: number
  amp: number
  layer: 0 | 1 | 2
}

// far, mid, near: [rMin, rMax, vyMin, vyMax, aMin, aMax]
const LAYER_PROFILE = [
  [0.6, 1.2, 14, 24, 0.4, 0.65],
  [1.3, 2.3, 26, 42, 0.65, 0.9],
  [2.6, 4.2, 48, 72, 0.3, 0.5]
] as const

// Cumulative share of flakes per layer, by layer count.
const LAYER_SPLIT: Record<number, readonly number[]> = {
  3: [0.55, 0.88, 1],
  2: [0.7, 1, 1],
  1: [0, 1, 1]
}

/**
 * Merges the theme's options over the defaults, ignoring unset ones. A size
 * set on one side keeps the range valid against the other side's default.
 */
export function resolveSnowOptions(
  opts: Partial<{ [K in keyof SnowOptions]: SnowOptions[K] | null | undefined }>
): SnowOptions {
  const o: SnowOptions = { ...SNOW_DEFAULTS }
  for (const key of Object.keys(SNOW_DEFAULTS) as (keyof SnowOptions)[]) {
    const v = opts[key]
    if (v !== undefined && v !== null) (o as Record<string, unknown>)[key] = v
  }
  const minSet = opts.minSize != null
  const maxSet = opts.maxSize != null
  if (minSet && !maxSet && o.minSize > o.maxSize) o.maxSize = o.minSize
  if (maxSet && !minSet && o.maxSize < o.minSize) o.minSize = o.maxSize
  return o
}

// The sprite is drawn so its solid core has the flake's radius; near flakes
// get a wide soft halo instead of the mock's per-frame shadowBlur.
const SPRITE_PX = 64
const CORE_FRACTION = [0.85, 0.85, 0.4] as const

/** Pre-renders the soft disc for one layer and colour ("R G B"). */
export function createFlakeSprite(
  layer: number,
  color: string
): CanvasImageSource {
  const canvas = document.createElement("canvas")
  canvas.width = SPRITE_PX
  canvas.height = SPRITE_PX
  const ctx = canvas.getContext("2d")
  if (!ctx) return canvas
  const half = SPRITE_PX / 2
  const core = CORE_FRACTION[Math.min(2, Math.max(0, layer))]
  const g = ctx.createRadialGradient(half, half, 0, half, half, half)
  if (layer >= 2) {
    // solid core, then a long fade out to the sprite's edge
    g.addColorStop(0, rgba(color, 1))
    g.addColorStop(core, rgba(color, 0.9))
    g.addColorStop(core + 0.15, rgba(color, 0.35))
    g.addColorStop(1, rgba(color, 0))
  } else {
    g.addColorStop(0, rgba(color, 1))
    g.addColorStop(core, rgba(color, 0.9))
    g.addColorStop(1, rgba(color, 0))
  }
  ctx.fillStyle = g
  ctx.fillRect(0, 0, SPRITE_PX, SPRITE_PX)
  return canvas
}

export function createSnow(
  opts: Parameters<typeof resolveSnowOptions>[0],
  ctx: CanvasRenderingContext2D,
  deps: {
    random(): number
    sprite(layer: number, color: string): CanvasImageSource
  }
): Effect & { flakes(): readonly Flake[] } {
  const o = resolveSnowOptions(opts)
  const split = LAYER_SPLIT[Math.round(o.layers)] ?? LAYER_SPLIT[3]
  const sizeScale =
    (o.maxSize - o.minSize) / (SNOW_DEFAULTS.maxSize - SNOW_DEFAULTS.minSize)
  const sprites: CanvasImageSource[] = []
  const spriteFor = (layer: number) =>
    (sprites[layer] ??= deps.sprite(layer, o.color))
  let width = 0
  let height = 0
  let flakes: Flake[] = []

  const build = () => {
    const rnd = deps.random
    const count = flakeCount(o.density, width, height)
    flakes = []
    for (let i = 0; i < count; i++) {
      const z = rnd()
      const layer = z < split[0] ? 0 : z < split[1] ? 1 : 2
      const [r0, r1, v0, v1, a0, a1] = LAYER_PROFILE[layer]
      const r =
        o.minSize + (r0 + rnd() * (r1 - r0) - SNOW_DEFAULTS.minSize) * sizeScale
      flakes.push({
        x: rnd() * width,
        y: rnd() * height,
        r,
        vy: (v0 + rnd() * (v1 - v0)) * o.speed,
        alpha: a0 + rnd() * (a1 - a0),
        phase: rnd() * 6.28,
        freq: 0.4 + rnd() * 0.9,
        amp: 8 + rnd() * 18 * (layer + 1),
        layer
      })
    }
  }

  const draw = () => {
    ctx.clearRect(0, 0, width, height)
    for (const f of flakes) {
      const half = f.r / CORE_FRACTION[f.layer]
      ctx.globalAlpha = f.alpha
      ctx.drawImage(
        spriteFor(f.layer),
        f.x - half,
        f.y - half,
        half * 2,
        half * 2
      )
    }
    ctx.globalAlpha = 1
  }

  return {
    fps: SNOW_FPS,
    flakes: () => flakes,
    resize(w, h) {
      width = Math.max(0, Math.round(w))
      height = Math.max(0, Math.round(h))
      // CSS px x 1: no devicePixelRatio scaling for snow.
      ctx.canvas.width = width
      ctx.canvas.height = height
      build()
      draw()
    },
    frame(timeMs, dt) {
      for (const f of flakes) {
        f.y += f.vy * dt
        f.x +=
          Math.sin((timeMs / 1000) * f.freq + f.phase) * f.amp * dt +
          o.wind * dt
        if (f.y > height + 6) {
          f.y = -6
          f.x = deps.random() * width
        }
        if (f.x > width + 6) f.x = -6
        else if (f.x < -6) f.x = width + 6
      }
      draw()
    }
  }
}
