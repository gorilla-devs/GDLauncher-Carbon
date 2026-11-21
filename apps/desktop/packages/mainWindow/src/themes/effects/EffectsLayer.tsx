import {
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
  Show,
  untrack,
  JSX
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { activeTheme } from "../active"
import { motion } from "../motion"
import { browserLoopDeps, createEffectsLoop } from "./loop"
import { createSnow, createFlakeSprite } from "./snow"
import { createLights, lightsHeight, LIGHTS_TOP } from "./lights"
import { createParticles } from "./particles"
import { createSpriteCache, rasterizeSprite } from "./sprites"

type Resizer = (width: number, height: number) => void

// Snow and string lights, drawn on canvases that fill the content column. It
// sits after the page in the column's isolated stacking context, so no page
// z-index can rise above it. Motion: "run" animates, "pause" keeps the last
// frame and schedules nothing, "off" has no snow and draws the lights once.
export function EffectsLayer(): JSX.Element {
  // The "Theme effects" switch: nothing is drawn until the preference has
  // loaded and says yes.
  const preferences = rspc.createQuery(() => ({
    queryKey: ["themes.getPreferences"]
  }))
  const effectsOn = () => preferences.data?.effects === true
  const snowCfg = createMemo(() =>
    effectsOn()
      ? activeTheme()?.effects.find((e) => e.type === "snow")
      : undefined
  )
  const lightsCfg = createMemo(() =>
    effectsOn()
      ? activeTheme()?.effects.find((e) => e.type === "stringLights")
      : undefined
  )
  const particlesCfg = createMemo(() =>
    effectsOn()
      ? activeTheme()?.effects.find((e) => e.type === "particles")
      : undefined
  )
  const hasEffects = () => !!snowCfg() || !!lightsCfg() || !!particlesCfg()
  // One seed per applied theme: the lights keep their sag and twinkle phases
  // across window resizes and motion changes.
  const seed = createMemo(() => {
    activeTheme()
    return Math.floor(Math.random() * 2 ** 32)
  })
  const isOff = createMemo(() => motion() === "off")
  const isRun = createMemo(() => motion() === "run")

  const [root, setRoot] = createSignal<HTMLDivElement>()
  const [snowCanvas, setSnowCanvas] = createSignal<HTMLCanvasElement>()
  const [lightsCanvas, setLightsCanvas] = createSignal<HTMLCanvasElement>()
  const [particlesCanvas, setParticlesCanvas] =
    createSignal<HTMLCanvasElement>()
  const [size, setSize] = createSignal({ width: 0, height: 0 })

  const loop = createEffectsLoop(browserLoopDeps)
  const resizers = new Set<Resizer>()

  // One ResizeObserver, only while there is something to draw.
  createEffect(() => {
    const el = root()
    if (!el || !hasEffects()) return
    const observer = new ResizeObserver((entries) => {
      const box = entries[entries.length - 1]?.contentRect
      if (!box) return
      const width = Math.round(box.width)
      const height = Math.round(box.height)
      const prev = size()
      if (prev.width !== width || prev.height !== height) {
        setSize({ width, height })
      }
    })
    observer.observe(el)
    onCleanup(() => observer.disconnect())
  })

  createEffect(
    on(size, ({ width, height }) => {
      if (width <= 0 || height <= 0) return
      resizers.forEach((r) => r(width, height))
    })
  )

  createEffect(() => {
    const cfg = snowCfg()
    const canvas = snowCanvas()
    if (cfg?.type !== "snow" || isOff() || !canvas) return
    const ctx = canvas.getContext("2d")
    if (!ctx) return
    const snow = createSnow(cfg, ctx, {
      random: Math.random,
      sprite: createFlakeSprite
    })
    const remove = loop.add(snow)
    resizers.add(snow.resize)
    const { width, height } = untrack(size)
    if (width > 0 && height > 0) snow.resize(width, height)
    onCleanup(() => {
      remove()
      resizers.delete(snow.resize)
    })
  })

  // Particles: the sprites load once per image; the effect draws as soon as
  // they are ready, so a paused layer still shows them.
  createEffect(() => {
    const cfg = particlesCfg()
    const canvas = particlesCanvas()
    if (cfg?.type !== "particles" || isOff() || !canvas) return
    const ctx = canvas.getContext("2d")
    if (!ctx) return
    const sprites = createSpriteCache((url) => rasterizeSprite(url, cfg))
    const particles = createParticles(
      {
        ...cfg,
        // the core validates the preset; anything else falls back to fall
        preset:
          cfg.preset === "float" || cfg.preset === "drift"
            ? cfg.preset
            : "fall",
        spriteCount: cfg.images.length
      },
      ctx,
      {
        random: Math.random,
        sprite: (i) => {
          const url = cfg.images[i]
          return url === undefined ? null : sprites.get(url)
        }
      }
    )
    // The loop entry exists only while there is something to draw: no
    // particles, or every sprite failed, and nothing clears a canvas at 30 fps.
    let remove: (() => void) | null = null
    let loaded = false
    const sync = () => {
      const wanted =
        loaded &&
        particles.particles().length > 0 &&
        cfg.images.some((url) => sprites.get(url) !== null)
      if (wanted && !remove) remove = loop.add(particles)
      else if (!wanted && remove) {
        remove()
        remove = null
      }
    }
    const resize: Resizer = (w, h) => {
      particles.resize(w, h)
      sync()
    }
    resizers.add(resize)
    const { width, height } = untrack(size)
    if (width > 0 && height > 0) resize(width, height)
    let live = true
    void Promise.all(cfg.images.map((url) => sprites.ready(url))).then(() => {
      if (!live) return
      loaded = true
      particles.draw()
      sync()
    })
    onCleanup(() => {
      live = false
      remove?.()
      resizers.delete(resize)
    })
  })

  createEffect(() => {
    const cfg = lightsCfg()
    const canvas = lightsCanvas()
    if (cfg?.type !== "stringLights" || !canvas) return
    const ctx = canvas.getContext("2d")
    if (!ctx) return
    const lights = createLights(cfg, ctx, { random: Math.random, seed: seed() })
    const off = isOff()
    const remove = off ? () => {} : loop.add(lights)
    const resize: Resizer = (width) => lights.resize(width, lightsHeight(cfg))
    resizers.add(resize)
    if (off) lights.drawStatic()
    const { width, height } = untrack(size)
    if (width > 0 && height > 0) resize(width, height)
    onCleanup(() => {
      remove()
      resizers.delete(resize)
    })
  })

  // setRunning(false) cancels the pending timer and raf outright.
  createEffect(() => loop.setRunning(hasEffects() && isRun()))
  onCleanup(() => loop.setRunning(false))

  return (
    <Show when={hasEffects()}>
      <div
        ref={setRoot}
        aria-hidden="true"
        class="pointer-events-none absolute inset-0 z-[1]"
      >
        <Show when={!!lightsCfg()}>
          <canvas
            ref={setLightsCanvas}
            data-gdl-effect="string-lights"
            class="absolute left-0 w-full"
            style={{
              top: `${LIGHTS_TOP}px`,
              height: `${lightsHeight(lightsCfg() ?? {})}px`
            }}
          />
        </Show>
        <Show when={!!particlesCfg() && !isOff()}>
          <canvas
            ref={setParticlesCanvas}
            data-gdl-effect="particles"
            class="absolute inset-0 h-full w-full"
          />
        </Show>
        <Show when={!!snowCfg() && !isOff()}>
          <canvas
            ref={setSnowCanvas}
            data-gdl-effect="snow"
            class="absolute inset-0 h-full w-full"
          />
        </Show>
      </div>
    </Show>
  )
}
