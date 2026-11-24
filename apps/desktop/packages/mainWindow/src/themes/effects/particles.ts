import type { Effect } from "./loop"

export const PARTICLES_DEFAULTS = {
  density: 6,
  speed: 1,
  minSize: 14,
  maxSize: 28,
  rotate: true,
  opacity: 0.9
} as const

export const MAX_PARTICLES = 150
export const PARTICLES_FPS = 30

export type ParticlePreset = "fall" | "float" | "drift"

export function particleCount(
  density: number,
  width: number,
  height: number
): number {
  return Math.min(
    MAX_PARTICLES,
    Math.max(0, Math.round((density * width * height) / 100_000))
  )
}

export interface Particle {
  x: number
  y: number
  size: number
  vx: number
  vy: number
  /** radians */
  angle: number
  /** degrees per second */
  spin: number
  phase: number
  freq: number
  amp: number
  sprite: number
  alpha: number
}

type Range = readonly [number, number]

interface Profile {
  vx: Range
  vy: Range
  amp: Range
  freq: Range
  spin: Range
}

// Per second, before the speed factor.
const PROFILES: Record<ParticlePreset, Profile> = {
  fall: {
    vx: [0, 0],
    vy: [30, 60],
    amp: [14, 30],
    freq: [0.5, 1.1],
    spin: [15, 60]
  },
  float: {
    vx: [0, 0],
    vy: [-30, -14],
    amp: [8, 16],
    freq: [0.3, 0.7],
    spin: [5, 20]
  },
  drift: {
    vx: [28, 56],
    vy: [3, 9],
    amp: [6, 14],
    freq: [0.3, 0.6],
    spin: [20, 80]
  }
}

/**
 * The size range the particles are drawn at, defaults applied. A size set on
 * one side keeps the range valid against the other side's default. The drawn
 * width is a size in this range; the sprite is rasterised to the largest.
 */
export function resolveSizeRange(opts: {
  minSize?: number | null
  maxSize?: number | null
}): { min: number; max: number } {
  let min = opts.minSize ?? PARTICLES_DEFAULTS.minSize
  let max = opts.maxSize ?? PARTICLES_DEFAULTS.maxSize
  if (min > max) {
    if (opts.minSize != null && opts.maxSize == null) max = min
    else min = max
  }
  return { min, max }
}

/**
 * The pixel size a sprite is rasterised at: as wide as the largest drawn
 * width, with the image's own aspect for the height, so nothing is upscaled
 * and a huge source costs nothing per frame.
 */
export function spriteRasterSize(
  opts: { minSize?: number | null; maxSize?: number | null },
  naturalWidth: number,
  naturalHeight: number
): { width: number; height: number } {
  const width = Math.max(1, Math.ceil(resolveSizeRange(opts).max))
  const aspect =
    naturalWidth > 0 && naturalHeight > 0 ? naturalHeight / naturalWidth : 1
  // a 1 x 4000 source must not ask for a 96 x 384000 raster
  const maxHeight = Math.min(4 * width, 512)
  return {
    width,
    height: Math.min(maxHeight, Math.max(1, Math.round(width * aspect)))
  }
}

interface Opts {
  preset: ParticlePreset
  spriteCount: number
  density?: number | null
  speed?: number | null
  minSize?: number | null
  maxSize?: number | null
  rotate?: boolean | null
  opacity?: number | null
}

export function createParticles(
  opts: Opts,
  ctx: CanvasRenderingContext2D,
  deps: {
    random(): number
    sprite(index: number): CanvasImageSource | null
  }
): Effect & { particles(): readonly Particle[]; draw(): void } {
  const D = PARTICLES_DEFAULTS
  const density = opts.density ?? D.density
  const speed = opts.speed ?? D.speed
  const { min: minSize, max: maxSize } = resolveSizeRange(opts)
  const rotate = opts.rotate ?? D.rotate
  const opacity = opts.opacity ?? D.opacity
  const profile = PROFILES[opts.preset]
  const rnd = deps.random
  const between = ([a, b]: Range) => a + rnd() * (b - a)

  let width = 0
  let height = 0
  let list: Particle[] = []

  // Each sprite's height/width, read once when its image is ready (not per
  // particle per frame).
  const aspects: number[] = []
  const aspectOf = (index: number): number => {
    let a = aspects[index]
    if (a === undefined) {
      const img = deps.sprite(index) as {
        width?: number
        height?: number
      } | null
      if (!img) return 1
      a = img.width && img.height ? img.height / img.width : 1
      aspects[index] = a
    }
    return a
  }
  // how far the drawn sprite reaches from its centre: respawns use it, so
  // tall sprites enter and leave fully off screen
  const reach = (p: Particle) => p.size * Math.max(1, aspectOf(p.sprite))

  // float fades in and out over its run from the bottom edge to the top
  const alphaOf = (p: Particle) => {
    if (opts.preset !== "float") return opacity
    const r = reach(p)
    const run = (height + r - p.y) / (height + 2 * r)
    return opacity * Math.sin(Math.PI * Math.min(1, Math.max(0, run)))
  }

  const build = () => {
    const count =
      opts.spriteCount > 0 ? particleCount(density, width, height) : 0
    list = []
    for (let i = 0; i < count; i++) {
      const size = minSize + rnd() * (maxSize - minSize)
      const dir = rnd() < 0.5 ? -1 : 1
      const p: Particle = {
        x: rnd() * width,
        y: rnd() * height,
        size,
        vx: between(profile.vx) * speed,
        vy: between(profile.vy) * speed,
        angle: rnd() * Math.PI * 2,
        spin: rotate ? dir * between(profile.spin) * speed : 0,
        phase: rnd() * Math.PI * 2,
        freq: between(profile.freq),
        amp: between(profile.amp) * speed,
        sprite: Math.min(
          opts.spriteCount - 1,
          Math.floor(rnd() * opts.spriteCount)
        ),
        alpha: opacity
      }
      if (!rotate) p.angle = 0
      p.alpha = alphaOf(p)
      list.push(p)
    }
  }

  const draw = () => {
    ctx.clearRect(0, 0, width, height)
    for (const p of list) {
      const img = deps.sprite(p.sprite)
      if (!img) continue
      const h = p.size * aspectOf(p.sprite)
      ctx.setTransform(1, 0, 0, 1, p.x, p.y)
      if (p.angle !== 0) ctx.rotate(p.angle)
      ctx.globalAlpha = p.alpha
      ctx.drawImage(img, -p.size / 2, -h / 2, p.size, h)
    }
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.globalAlpha = 1
  }

  return {
    fps: PARTICLES_FPS,
    particles: () => list,
    draw,
    resize(w, h) {
      width = Math.max(0, Math.round(w))
      height = Math.max(0, Math.round(h))
      // CSS px x 1: no devicePixelRatio scaling
      ctx.canvas.width = width
      ctx.canvas.height = height
      build()
      draw()
    },
    frame(timeMs, dt) {
      const t = timeMs / 1000
      for (const p of list) {
        const sway = Math.sin(t * p.freq + p.phase) * p.amp * dt
        if (opts.preset === "drift") {
          p.x += p.vx * dt
          p.y += p.vy * dt + sway
        } else {
          p.x += sway
          p.y += p.vy * dt
        }
        p.angle += (p.spin * Math.PI * dt) / 180

        const r = reach(p)
        if (opts.preset === "fall" && p.y > height + r) {
          p.y = -r
          p.x = rnd() * width
        } else if (opts.preset === "float" && p.y < -r) {
          p.y = height + r
          p.x = rnd() * width
        } else if (opts.preset === "drift") {
          if (p.x > width + r) {
            p.x = -r
            p.y = rnd() * height
          }
          if (p.y > height + r) p.y = -r
          else if (p.y < -r) p.y = height + r
        }
        if (opts.preset !== "drift") {
          if (p.x > width + r) p.x = -r
          else if (p.x < -r) p.x = width + r
        }
        p.alpha = alphaOf(p)
      }
      draw()
    }
  }
}
