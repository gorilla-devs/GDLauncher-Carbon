import { describe, it, expect } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { generateSilentNight } from "./generate.mjs"
import { VARIANTS } from "./variants.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))
const assets = path.resolve(here, "../../themes/silent-night/assets")

const walk = (dir, base = "") =>
  fs.existsSync(dir)
    ? fs
        .readdirSync(dir, { withFileTypes: true })
        .flatMap((e) =>
          e.isDirectory()
            ? walk(path.join(dir, e.name), `${base}${e.name}/`)
            : [`${base}${e.name}`]
        )
    : []

/** On-curve points of a path: M/L end points and the end point of each C. */
const onCurveYs = (d) => {
  const ys = []
  for (const [, cmd, args] of d.matchAll(/([MLCZHV])([^MLCZHV]*)/g)) {
    const n = (args.match(/-?\d*\.?\d+/g) || []).map(Number)
    if (cmd === "M" || cmd === "L") ys.push(n[1])
    else if (cmd === "H") continue
    else if (cmd === "V") ys.push(n[0])
    else if (cmd === "C") {
      for (let i = 0; i + 5 < n.length + 1; i += 6) ys.push(n[i + 5])
    }
  }
  return ys
}

const mainPath = (svg) => {
  const m = svg.match(/<path d="([^"]+)" fill="url\(#sf\)"/)
  if (!m) throw new Error("no main path")
  return m[1]
}

/** Top y of the clumps (ellipses) and glints (white four-point stars). */
const extraTops = (svg) => {
  const tops = [30]
  for (const [, cy, ry] of svg.matchAll(
    /<ellipse [^>]*cy="([^"]+)"[^>]*ry="([^"]+)"/g
  ))
    tops.push(Number(cy) - Number(ry))
  for (const [, y] of svg.matchAll(
    /<path fill="#ffffff" opacity="0.9" d="M[\d.]+ ([\d.]+)/g
  ))
    tops.push(Number(y))
  return tops
}

describe("silent night generator", () => {
  const files = generateSilentNight()

  it("the committed assets match the generator", () => {
    for (const [rel, data] of files) {
      if (typeof data === "string")
        expect(fs.readFileSync(path.join(assets, rel), "utf8"), rel).toBe(data)
      else
        expect(
          Buffer.compare(
            fs.readFileSync(path.join(assets, rel)),
            Buffer.from(data)
          ),
          rel
        ).toBe(0)
    }
    expect(walk(assets).sort()).toEqual([...files.keys()].sort())
  })

  it("it is deterministic", () => {
    expect([...generateSilentNight()]).toEqual([...generateSilentNight()])
  })

  it("there are at least 12 varied caps", () => {
    const caps = [...files.keys()].filter((k) => k.startsWith("caps/"))
    expect(caps).toHaveLength(16)
    expect(VARIANTS).toHaveLength(16)
    const dust = VARIANTS.filter((v) => v.thick >= 4 && v.thick <= 6.5)
    const medium = VARIANTS.filter((v) => v.thick >= 10 && v.thick <= 16)
    const heavy = VARIANTS.filter((v) => v.thick >= 17 && v.thick <= 20)
    expect([dust.length, medium.length, heavy.length]).toEqual([3, 7, 6])
    for (const k of [1, 2, 3, 4]) {
      expect(
        VARIANTS.filter((v) => v.mounds === k).length,
        `mounds ${k}`
      ).toBeGreaterThanOrEqual(3)
    }
    expect(VARIANTS.filter((v) => v.spill === "l")).toHaveLength(4)
    expect(VARIANTS.filter((v) => v.spill === "r")).toHaveLength(4)
    expect(VARIANTS.filter((v) => v.icicles > 0).length).toBeGreaterThanOrEqual(
      6
    )
    expect(VARIANTS.filter((v) => v.glints > 0).length).toBeGreaterThanOrEqual(
      8
    )
  })

  it("caps never reach the row above", () => {
    for (const [rel, text] of files) {
      if (!rel.startsWith("caps/")) continue
      const ys = onCurveYs(mainPath(text))
      expect(Math.min(...ys), rel).toBeGreaterThanOrEqual(10)
      expect(
        Math.min(...extraTops(text)),
        `${rel} clumps/glints`
      ).toBeGreaterThanOrEqual(10)
    }
  })

  it("the ledge stays within the 16 px above the panel", () => {
    const text = files.get("ledge.svg")
    const ys = onCurveYs(mainPath(text))
    expect(Math.min(...ys)).toBeGreaterThanOrEqual(30 - 16)
    expect(Math.min(...extraTops(text))).toBeGreaterThanOrEqual(30 - 16)
  })

  it("the hat lies inside its viewBox", () => {
    const svg = files.get("hat.svg")
    const [vx, vy, vw, vh] = svg
      .match(/viewBox="([^"]+)"/)[1]
      .split(" ")
      .map(Number)
    expect(svg).toContain(`width="${vw}" height="${vh}"`)
    // translate(0 -4) after rotate(-7 28 18)
    const a = (-7 * Math.PI) / 180
    const tf = (x, y) => {
      const dx = x - 28
      const dy = y - 18
      return [
        28 + dx * Math.cos(a) - dy * Math.sin(a),
        18 + dx * Math.sin(a) + dy * Math.cos(a) - 4
      ]
    }
    const pts = []
    for (const [, d] of svg.matchAll(/<path d="([^"]+)"/g)) {
      const n = (d.match(/-?\d*\.?\d+/g) || []).map(Number)
      for (let i = 0; i + 1 < n.length; i += 2) pts.push(tf(n[i], n[i + 1]))
    }
    for (const [, cx, cy, r] of svg.matchAll(
      /<circle cx="([^"]+)" cy="([^"]+)" r="([^"]+)"/g
    )) {
      for (const [ox, oy] of [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1]
      ])
        pts.push(tf(+cx + ox * r, +cy + oy * r))
    }
    expect(pts.length).toBeGreaterThan(40)
    for (const [x, y] of pts) {
      expect(x).toBeGreaterThanOrEqual(vx)
      expect(x).toBeLessThanOrEqual(vx + vw)
      expect(y).toBeGreaterThanOrEqual(vy)
      expect(y).toBeLessThanOrEqual(vy + vh)
    }
  })

  it("assets are plain images", () => {
    const svgs = walk(assets).filter((r) => r.endsWith(".svg"))
    expect(svgs).toHaveLength(16 + 8)
    for (const rel of svgs) {
      const text = fs.readFileSync(path.join(assets, rel), "utf8")
      expect(text, rel).toMatch(/^<svg xmlns="http:\/\/www.w3.org\/2000\/svg"/)
      expect(text, rel).not.toMatch(/<script|href="http|data:|javascript:/i)
      expect(text, rel).not.toMatch(/url\((?!#)/)
    }
  })

  it("the sounds are short, quiet PCM WAVs", () => {
    for (const rel of ["sounds/sleigh-bells.wav", "sounds/chime.wav"]) {
      const b = Buffer.from(files.get(rel))
      expect(b.toString("latin1", 0, 4), rel).toBe("RIFF")
      expect(b.toString("latin1", 8, 12), rel).toBe("WAVE")
      expect(b.toString("latin1", 12, 16), rel).toBe("fmt ")
      expect(b.readUInt16LE(20)).toBe(1)
      expect(b.readUInt16LE(22)).toBe(1)
      expect(b.readUInt32LE(24)).toBe(22050)
      expect(b.readUInt16LE(34)).toBe(16)
      expect(b.toString("latin1", 36, 40)).toBe("data")
      const n = b.readUInt32LE(40) / 2
      expect(b.length).toBe(44 + n * 2)
      expect(n / 22050, rel).toBeLessThanOrEqual(2.0)
      let peak = 0
      let sum = 0
      let sq = 0
      for (let i = 0; i < n; i++) {
        const v = b.readInt16LE(44 + i * 2)
        peak = Math.max(peak, Math.abs(v))
        sum += v
        sq += v * v
      }
      expect(peak, rel).toBeLessThanOrEqual(0.25 * 32767)
      expect(Math.sqrt(sq / n), rel).toBeGreaterThan(0.005 * 32767)
      expect(Math.abs(sum / n), `${rel} dc offset`).toBeLessThan(0.002 * 32767)
    }
  })

  it("the motion layer steps in whole 250 ms and respects reduced motion", () => {
    const motion = files.get("scene-motion.svg")
    expect(motion).toContain(
      "@media (prefers-reduced-motion:reduce){*{animation:none!important}}"
    )
    // keyframes: name -> [{ pct, timing }]
    const frames = {}
    for (const [, name, body] of motion.matchAll(
      /@keyframes (\w+)\{((?:[\d.%,]+\{[^}]*\})+)\}/g
    )) {
      frames[name] = []
      for (const [, sel, decl] of body.matchAll(/([\d.%,]+)\{([^}]*)\}/g)) {
        const timing = decl.match(/animation-timing-function:\s*steps\((\d+)/)
        for (const p of sel.split(","))
          frames[name].push({
            pct: parseFloat(p),
            steps: timing ? +timing[1] : null
          })
      }
      frames[name].sort((x, y) => x.pct - y.pct)
    }
    expect(Object.keys(frames).sort()).toEqual(["gdBulb", "gdSmoke", "gdTw"])
    const anims = [...motion.matchAll(/(?<![-\w])animation\s*:\s*([^;"}]+)/g)]
    expect(anims.length).toBeGreaterThan(50)
    for (const [, v] of anims) {
      if (v.startsWith("none")) continue // the reduced-motion rule
      const m = v.match(
        /^(\w+) (-?[\d.]+)s steps\((\d+),end\) (-?[\d.]+)s infinite$/
      )
      expect(m, v).not.toBeNull()
      const [, name, dur, elSteps, delay] = m
      const durMs = Number(dur) * 1000
      expect(Math.abs(durMs % 250), v).toBe(0)
      expect(Math.abs((Number(delay) * 1000) % 250), v).toBe(0)
      const kf = frames[name]
      expect(kf, name).toBeDefined()
      for (let i = 0; i + 1 < kf.length; i++) {
        const segMs = ((kf[i + 1].pct - kf[i].pct) / 100) * durMs
        const steps = kf[i].steps ?? Number(elSteps)
        expect(
          Math.abs(segMs / steps - 250),
          `${name} segment ${i} of ${v}`
        ).toBeLessThan(0.01)
      }
    }
    expect(files.get("scene.svg")).not.toMatch(/animation|<animate/)
  })

  it("sizes stay small", () => {
    expect(files.get("scene.svg").length).toBeLessThan(400 * 1024)
    for (const rel of ["sounds/sleigh-bells.wav", "sounds/chime.wav"])
      expect(files.get(rel).length, rel).toBeLessThan(128 * 1024)
    for (const [rel, text] of files) {
      if (rel.startsWith("caps/"))
        expect(text.length, rel).toBeLessThan(12 * 1024)
    }
  })
})
