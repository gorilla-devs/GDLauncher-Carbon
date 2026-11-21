import { rng } from "./rng.mjs"

const RATE = 22050

/** 16-bit mono PCM WAV (44-byte header) from samples in -1..1. */
export function wav(samples, rate = RATE) {
  const n = samples.length
  const out = new Uint8Array(44 + n * 2)
  const v = new DataView(out.buffer)
  const tag = (at, s) => {
    for (let i = 0; i < 4; i++) out[at + i] = s.charCodeAt(i)
  }
  tag(0, "RIFF")
  v.setUint32(4, 36 + n * 2, true)
  tag(8, "WAVE")
  tag(12, "fmt ")
  v.setUint32(16, 16, true)
  v.setUint16(20, 1, true) // PCM
  v.setUint16(22, 1, true) // mono
  v.setUint32(24, rate, true)
  v.setUint32(28, rate * 2, true)
  v.setUint16(32, 2, true)
  v.setUint16(34, 16, true)
  tag(36, "data")
  v.setUint32(40, n * 2, true)
  for (let i = 0; i < n; i++) {
    const x = Math.max(-1, Math.min(1, samples[i]))
    v.setInt16(44 + i * 2, Math.round(x * 32767), true)
  }
  return out
}

/** Scales to the given peak, then fades linearly to silence over `fade` s. */
function finish(buf, peak, fade) {
  let max = 0
  for (const x of buf) max = Math.max(max, Math.abs(x))
  const g = max > 0 ? peak / max : 0
  const fadeN = Math.round(fade * RATE)
  for (let i = 0; i < buf.length; i++) {
    const left = buf.length - i
    buf[i] *= g * (left < fadeN ? left / fadeN : 1)
  }
  return buf
}

/**
 * Adds one decaying sine partial. A short linear attack keeps the start free
 * of clicks.
 */
function partial(buf, start, freq, amp, tau, attack) {
  const s0 = Math.round(start * RATE)
  const len = Math.min(buf.length - s0, Math.round(tau * 9 * RATE))
  const atkN = Math.max(1, Math.round(attack * RATE))
  for (let i = 0; i < len; i++) {
    const t = i / RATE
    const env = Math.exp(-t / tau) * (i < atkN ? i / atkN : 1)
    buf[s0 + i] += amp * env * Math.sin(2 * Math.PI * freq * t)
  }
}

const BELL_STRIKES = [
  0, 0.06, 0.2, 0.26, 0.4, 0.46, 0.6, 0.66, 0.8, 0.86, 1, 1.06
]
const BELL_BASES = [2637, 2794, 3136]
const BELL_RATIOS = [1, 1.5, 2.0, 2.68]
const BELL_AMPS = [1, 0.6, 0.45, 0.3]
const BELL_TAUS = [1, 0.8, 0.6, 0.5].map((k) => 0.12 * k)

/** A shake of small bells: pairs of strikes, loud then soft. 1.6 s. */
export function sleighBells() {
  const rand = rng(1224)
  const buf = new Float32Array(Math.round(1.6 * RATE))
  BELL_STRIKES.forEach((at, i) => {
    const gain = i % 2 === 0 ? 0.9 : 0.6
    const base =
      BELL_BASES[Math.floor(rand() * BELL_BASES.length)] *
      (1 + (rand() * 2 - 1) * 0.01)
    BELL_RATIOS.forEach((r, k) =>
      partial(buf, at, base * r, gain * BELL_AMPS[k], BELL_TAUS[k], 0.002)
    )
    // a 5 ms noise click, eased in over its first half millisecond
    const s0 = Math.round(at * RATE)
    const clickN = Math.round(0.005 * RATE)
    const rampN = Math.max(1, Math.round(0.0005 * RATE))
    for (let j = 0; j < clickN; j++) {
      const env = Math.exp(-j / RATE / 0.004) * (j < rampN ? j / rampN : 1)
      buf[s0 + j] += gain * 0.3 * env * (rand() * 2 - 1)
    }
  })
  return wav(finish(buf, 0.22, 0.15))
}

/** Two soft bell notes, E6 then B6. 1.4 s. */
export function chime() {
  const buf = new Float32Array(Math.round(1.4 * RATE))
  for (const [at, f] of [
    [0, 1318.51],
    [0.14, 1975.53]
  ])
    [
      [1, 1, 0.5],
      [2.76, 0.35, 0.25],
      [5.4, 0.12, 0.12]
    ].forEach(([r, a, tau]) => partial(buf, at, f * r, a, tau, 0.004))
  return wav(finish(buf, 0.2, 0.1))
}
