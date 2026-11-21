import { rgba, seededRandom } from "./color"
import type { Effect } from "./loop"

export const LIGHTS_DEFAULTS = {
  colors: [
    "255 240 200",
    "255 190 92",
    "255 240 200",
    "255 116 102",
    "255 240 200",
    "255 190 92",
    "255 240 200",
    "159 224 166"
  ],
  spacing: 162,
  droop: 17,
  size: 1.15,
  twinkle: 0.4,
  bulb: "teardrop"
} as const

export const LIGHTS_FPS = 20
export const LIGHTS_HEIGHT = 72
// The canvas hangs this far above the content column, so the wire (at WIRE_Y
// in canvas space) sits on the content's top edge: the nav's bottom edge.
export const LIGHTS_TOP = -8
const WIRE_Y = 8
const HOOK_INSET = 14
const BULB_T = [0.22, 0.5, 0.78] as const

// A bulb's glow, in bulb units (scaled by `size`), as in the approved mock: a
// disc of radius 10, 12 below the socket, blurred with sigma 4.5 and drawn
// at 0.55. GLOW_RADIUS is where the blur has faded out.
export const GLOW_RADIUS = 20
const GLOW_CENTER_Y = 12
const GLOW_DISC = 10
const GLOW_SIGMA = 4.5
const GLOW_ALPHA = 0.55
const GLOW_STOPS = 10

/**
 * A disc of radius `disc` blurred by a Gaussian of `sigma`, sampled `r` from
 * its centre: 1 deep inside, about a half at the edge, 0 far outside.
 */
export function blurredDisc(r: number, disc: number, sigma: number): number {
  // The share of the Gaussian around the sample point that lands in the disc,
  // summed in polar steps out to five sigma.
  const RHO_STEPS = 256
  const THETA_STEPS = 90
  const dRho = (sigma * 5) / RHO_STEPS
  const dTheta = (Math.PI * 2) / THETA_STEPS
  let sum = 0
  for (let i = 0; i < RHO_STEPS; i++) {
    const rho = (i + 0.5) * dRho
    const weight =
      Math.exp(-(rho * rho) / (2 * sigma * sigma)) * rho * dRho * dTheta
    for (let j = 0; j < THETA_STEPS; j++) {
      const x = r + rho * Math.cos(j * dTheta)
      const y = rho * Math.sin(j * dTheta)
      if (x * x + y * y <= disc * disc) sum += weight
    }
  }
  return sum / (Math.PI * 2 * sigma * sigma)
}

let glowProfile: number[] | null = null
// The glow's alpha at GLOW_STOPS + 1 even steps out to GLOW_RADIUS, the last
// forced to 0 so the sprite has no edge.
const glowAlphas = (): number[] => {
  glowProfile ??= Array.from({ length: GLOW_STOPS + 1 }, (_, k) =>
    k === GLOW_STOPS
      ? 0
      : blurredDisc((k / GLOW_STOPS) * GLOW_RADIUS, GLOW_DISC, GLOW_SIGMA)
  )
  return glowProfile
}

export interface Bulb {
  x: number
  y: number
  /** Degrees. */
  angle: number
  /** "R G B" */
  color: string
  /** Twinkle period in seconds. */
  period: number
  /** Twinkle phase in radians. */
  offset: number
}

export function layoutLights(
  width: number,
  opts: { spacing: number; droop: number; colors: readonly string[] },
  random: () => number
): { hooks: number[]; bulbs: Bulb[]; controlY: number[] } {
  const span = Math.max(0, width - HOOK_INSET * 2)
  const spacing = opts.spacing > 0 ? opts.spacing : LIGHTS_DEFAULTS.spacing
  // Whole spans only: the last hook lands exactly on width - 14.
  const spans = Math.max(1, Math.round(span / spacing))
  const step = span / spans
  const colors = opts.colors.length > 0 ? opts.colors : LIGHTS_DEFAULTS.colors
  const hooks: number[] = []
  for (let i = 0; i <= spans; i++) hooks.push(HOOK_INSET + i * step)
  const bulbs: Bulb[] = []
  const controlY: number[] = []
  for (let s = 0; s < spans; s++) {
    const x0 = hooks[s]
    const x1 = hooks[s + 1]
    const droop = opts.droop + (random() * 4 - 2)
    const cx = (x0 + x1) / 2
    const cy = WIRE_Y + droop * 2
    controlY.push(cy)
    for (const t of BULB_T) {
      const u = 1 - t
      bulbs.push({
        x: u * u * x0 + 2 * u * t * cx + t * t * x1,
        y: u * u * WIRE_Y + 2 * u * t * cy + t * t * WIRE_Y,
        angle: (t - 0.5) * 18,
        color: colors[bulbs.length % colors.length],
        period: 2.8 + random() * 3,
        offset: random() * Math.PI * 2
      })
    }
  }
  return { hooks, bulbs, controlY }
}

type Offscreen = HTMLCanvasElement | OffscreenCanvas

function defaultCreateCanvas(w: number, h: number): Offscreen {
  const c = document.createElement("canvas")
  c.width = w
  c.height = h
  return c
}

export type LightsOptions = Partial<{
  colors: readonly string[] | null
  spacing: number | null
  droop: number | null
  size: number | null
  twinkle: number | null
  bulb: string | null
}>

/**
 * Canvas height that holds the lowest bulb and its glow: the wire, the droop
 * (plus its +-2 jitter) and the glow below the socket (its centre plus its
 * radius, scaled by size), which reaches past the bulb body. Never below
 * LIGHTS_HEIGHT.
 */
export function lightsHeight(opts: LightsOptions): number {
  const droop = opts.droop ?? LIGHTS_DEFAULTS.droop
  const size = opts.size ?? LIGHTS_DEFAULTS.size
  return Math.max(
    LIGHTS_HEIGHT,
    Math.ceil(WIRE_Y + droop + 2 + (GLOW_CENTER_Y + GLOW_RADIUS) * size)
  )
}

export function createLights(
  opts: LightsOptions,
  ctx: CanvasRenderingContext2D,
  deps: {
    random(): number
    /** When set, every layout replays this seed instead of using random(). */
    seed?: number
    createCanvas?(w: number, h: number): Offscreen
  }
): Effect & { drawStatic(): void } {
  const colors: readonly string[] = opts.colors?.length
    ? opts.colors
    : LIGHTS_DEFAULTS.colors
  const spacing = opts.spacing ?? LIGHTS_DEFAULTS.spacing
  const droop = opts.droop ?? LIGHTS_DEFAULTS.droop
  const size = opts.size ?? LIGHTS_DEFAULTS.size
  const twinkle = opts.twinkle ?? LIGHTS_DEFAULTS.twinkle
  const round = opts.bulb === "round"
  const createCanvas = deps.createCanvas ?? defaultCreateCanvas
  const height = lightsHeight(opts)

  let width = 0
  let dpr = 1
  let bulbs: Bulb[] = []
  let wire: Offscreen | null = null
  let staticMode = false
  let lastTime = 0
  const glows = new Map<string, Offscreen>()

  const glowFor = (color: string): Offscreen => {
    let g = glows.get(color)
    if (!g) {
      // Twice the drawn size, so it stays smooth at device pixel ratio 2.
      const px = GLOW_RADIUS * 4
      g = createCanvas(px, px)
      const gc = g.getContext("2d") as CanvasRenderingContext2D | null
      if (gc) {
        const grad = gc.createRadialGradient(
          px / 2,
          px / 2,
          0,
          px / 2,
          px / 2,
          px / 2
        )
        glowAlphas().forEach((a, k) =>
          grad.addColorStop(k / GLOW_STOPS, rgba(color, a))
        )
        gc.fillStyle = grad
        gc.fillRect(0, 0, px, px)
      }
      glows.set(color, g)
    }
    return g
  }

  const buildWire = (hooks: number[], controlY: number[]) => {
    const off = createCanvas(
      Math.max(1, Math.round(width * dpr)),
      Math.round(height * dpr)
    )
    const wc = off.getContext("2d") as CanvasRenderingContext2D | null
    if (!wc) return off
    wc.scale(dpr, dpr)
    wc.strokeStyle = "#17140f"
    wc.lineWidth = 1.6
    for (let s = 0; s < hooks.length - 1; s++) {
      const x0 = hooks[s]
      const x1 = hooks[s + 1]
      const cy = controlY[s]
      wc.beginPath()
      wc.moveTo(x0, WIRE_Y)
      wc.quadraticCurveTo((x0 + x1) / 2, cy, x1, WIRE_Y)
      wc.stroke()
    }
    wc.fillStyle = "#2a2620"
    for (const x of hooks) {
      wc.beginPath()
      wc.arc(x, WIRE_Y, 2.2, 0, Math.PI * 2)
      wc.fill()
    }
    return off
  }

  const drawBulb = (b: Bulb, alpha: number) => {
    ctx.save()
    ctx.translate(b.x, b.y)
    ctx.rotate((b.angle * Math.PI) / 180)
    ctx.scale(size, size)
    ctx.globalAlpha = alpha * GLOW_ALPHA
    ctx.drawImage(
      glowFor(b.color),
      -GLOW_RADIUS,
      GLOW_CENTER_Y - GLOW_RADIUS,
      GLOW_RADIUS * 2,
      GLOW_RADIUS * 2
    )
    ctx.globalAlpha = 1
    ctx.fillStyle = "#21402c"
    ctx.beginPath()
    ctx.roundRect(-2.6, 0, 5.2, 5, 1)
    ctx.fill()
    ctx.globalAlpha = alpha
    ctx.fillStyle = rgba(b.color, 1)
    ctx.beginPath()
    if (round) {
      ctx.arc(0, 11, 6, 0, Math.PI * 2)
    } else {
      ctx.moveTo(-3.4, 5)
      ctx.bezierCurveTo(-5.4, 9, -4.6, 15.5, 0, 18)
      ctx.bezierCurveTo(4.6, 15.5, 5.4, 9, 3.4, 5)
      ctx.closePath()
    }
    ctx.fill()
    ctx.strokeStyle = "rgba(255,255,255,0.7)"
    ctx.lineWidth = 1.1
    ctx.lineCap = "round"
    ctx.beginPath()
    ctx.moveTo(-1.6, 7)
    ctx.bezierCurveTo(-2.6, 10, -2.2, 13, -0.6, 14.5)
    ctx.stroke()
    ctx.restore()
  }

  const draw = () => {
    if (width <= 0) return
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, width, height)
    if (wire) ctx.drawImage(wire, 0, 0, width, height)
    for (const b of bulbs) {
      const alpha = staticMode
        ? 1
        : 1 -
          twinkle *
            (0.5 +
              0.5 *
                Math.sin(2 * Math.PI * (lastTime / 1000 / b.period) + b.offset))
      drawBulb(b, alpha)
    }
    ctx.globalAlpha = 1
  }

  return {
    fps: LIGHTS_FPS,
    resize(w) {
      width = Math.max(0, Math.round(w))
      dpr = Math.min(globalThis.devicePixelRatio || 1, 2)
      ctx.canvas.width = Math.max(1, Math.round(width * dpr))
      ctx.canvas.height = Math.round(height * dpr)
      const layout = layoutLights(
        width,
        { spacing, droop, colors },
        deps.seed === undefined ? deps.random : seededRandom(deps.seed)
      )
      bulbs = layout.bulbs
      wire = buildWire(layout.hooks, layout.controlY)
      draw()
    },
    frame(timeMs) {
      staticMode = false
      lastTime = timeMs
      draw()
    },
    drawStatic() {
      staticMode = true
      draw()
    }
  }
}
