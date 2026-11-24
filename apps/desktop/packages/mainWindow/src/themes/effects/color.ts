/** "R G B" (spaces or commas) plus an alpha, as a CSS colour. */
export function rgba(color: string, alpha: number): string {
  const [r, g, b] = color.split(/[\s,]+/).map(Number)
  return `rgba(${r},${g},${b},${alpha})`
}

/** Small seeded generator (mulberry32), so a layout can be replayed. */
export function seededRandom(seed: number): () => number {
  let a = seed | 0
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
