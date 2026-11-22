import { describe, expect, it } from "vitest"
import {
  blockAt,
  dragged,
  fitAll,
  MAX_ZOOM,
  scaleFor,
  screenOf,
  visiblePieces,
  zoomAround,
  type View
} from "./mapView"

const square = (from: number, to: number) => {
  const out: { x: number; z: number }[] = []
  for (let z = from; z <= to; z++)
    for (let x = from; x <= to; x++) out.push({ x, z })
  return out
}

describe("scaleFor", () => {
  it.each([
    [8, 0],
    [1, 0],
    [0.75, 0],
    [0.74, 1],
    [0.3, 2],
    [0.1, 3],
    [0.01, 4]
  ])("at zoom %f uses scale %i", (zoom, scale) => {
    expect(scaleFor(zoom)).toBe(scale)
  })
})

describe("visiblePieces", () => {
  const origin: View = { centerX: 0, centerZ: 0, zoom: 1 }

  it("lists the four scale 0 pieces around the origin with their screen boxes", () => {
    const pieces = visiblePieces(origin, 1000, 600, square(-1, 0))
    expect([...pieces].sort((a, b) => a.z - b.z || a.x - b.x)).toEqual([
      { scale: 0, x: -1, z: -1, left: -12, top: -212, size: 512 },
      { scale: 0, x: 0, z: -1, left: 500, top: -212, size: 512 },
      { scale: 0, x: -1, z: 0, left: -12, top: 300, size: 512 },
      { scale: 0, x: 0, z: 0, left: 500, top: 300, size: 512 }
    ])
  })

  it("does not list a region far away", () => {
    const pieces = visiblePieces(origin, 1000, 600, [
      ...square(-1, 0),
      { x: 40, z: 40 }
    ])
    expect(pieces.map((p) => [p.x, p.z])).not.toContainEqual([40, 40])
    expect(pieces).toHaveLength(4)
  })

  it("does not ask regions that exist but are off screen", () => {
    // x = 1 starts at block 512, past the right edge at 500.
    const pieces = visiblePieces(origin, 1000, 600, [
      { x: 1, z: 0 },
      { x: 0, z: 1 },
      { x: 0, z: 0 }
    ])
    expect(pieces.map((p) => [p.x, p.z])).toEqual([[0, 0]])
  })

  it("asks no piece over an area without a region", () => {
    expect(visiblePieces(origin, 1000, 600, [{ x: 5, z: 5 }])).toEqual([])
    expect(visiblePieces(origin, 1000, 600, [])).toEqual([])
  })

  it("at zoom 0.1 uses scale 3 pieces, each over an existing region", () => {
    const regions = [
      { x: -6, z: 3 },
      { x: 0, z: 0 },
      { x: 9, z: -3 },
      // Under a piece entirely off screen.
      { x: -20, z: 3 }
    ]
    const pieces = visiblePieces(
      { centerX: 0, centerZ: 0, zoom: 0.1 },
      1000,
      600,
      regions
    )
    expect(pieces.every((p) => p.scale === 3)).toBe(true)
    // A scale 3 piece covers 8 × 8 regions.
    expect(pieces.map((p) => [p.x, p.z]).sort()).toEqual(
      [
        [-1, 0],
        [0, 0],
        [1, -1]
      ].sort()
    )
    for (const p of pieces) {
      expect(
        regions.some(
          (r) => Math.floor(r.x / 8) === p.x && Math.floor(r.z / 8) === p.z
        )
      ).toBe(true)
      expect(p.size).toBeCloseTo(512 * 8 * 0.1)
    }
  })

  it("lists the pieces nearest the centre first", () => {
    const pieces = visiblePieces(
      { centerX: 256, centerZ: 256, zoom: 0.8 },
      4000,
      4000,
      square(-3, 3)
    )
    expect([pieces[0].x, pieces[0].z]).toEqual([0, 0])
    const distance = (p: { left: number; top: number; size: number }) =>
      Math.hypot(p.left + p.size / 2 - 2000, p.top + p.size / 2 - 2000)
    for (let i = 1; i < pieces.length; i++)
      expect(distance(pieces[i])).toBeGreaterThanOrEqual(
        distance(pieces[i - 1]) - 1e-9
      )
  })

  it("never lists more than 64 pieces", () => {
    const pieces = visiblePieces(
      { centerX: 0, centerZ: 0, zoom: 1 },
      10000,
      10000,
      square(-20, 20)
    )
    expect(pieces).toHaveLength(64)
    expect([pieces[0].x, pieces[0].z]).toEqual([-1, -1])
  })
})

describe("zoomAround", () => {
  const limits = { min: 0.05, max: MAX_ZOOM }

  it("keeps the block under the pointer under the pointer", () => {
    const view: View = { centerX: 100, centerZ: -40, zoom: 1 }
    const under = (v: View) => ({
      x: v.centerX + (700 - 500) / v.zoom,
      z: v.centerZ + (150 - 300) / v.zoom
    })
    const zoomed = zoomAround(view, 1.25, 700, 150, 1000, 600, limits)
    expect(zoomed.zoom).toBeCloseTo(1.25)
    expect(under(zoomed).x).toBeCloseTo(under(view).x)
    expect(under(zoomed).z).toBeCloseTo(under(view).z)
  })

  it("clamps the zoom and still keeps the pointer's block", () => {
    const view: View = { centerX: 0, centerZ: 0, zoom: 6 }
    const zoomed = zoomAround(view, 4, 900, 100, 1000, 600, limits)
    expect(zoomed.zoom).toBe(MAX_ZOOM)
    expect(zoomed.centerX + 400 / zoomed.zoom).toBeCloseTo(400 / 6)
    expect(zoomAround(view, 0.0001, 0, 0, 1000, 600, limits).zoom).toBe(0.05)
  })
})

describe("dragged", () => {
  it("moves the centre against the drag, in blocks", () => {
    expect(dragged({ centerX: 10, centerZ: 10, zoom: 2 }, 40, -20)).toEqual({
      centerX: -10,
      centerZ: 20,
      zoom: 2
    })
  })
})

describe("fitAll", () => {
  it("fits one region", () => {
    expect(fitAll([{ x: 0, z: 0 }], 1000, 600)).toEqual({
      centerX: 256,
      centerZ: 256,
      zoom: 600 / 512
    })
  })

  it("fits a 40 × 3 strip by its width", () => {
    const strip: { x: number; z: number }[] = []
    for (let x = -10; x < 30; x++)
      for (let z = 0; z < 3; z++) strip.push({ x, z })
    const view = fitAll(strip, 1000, 600)
    expect(view.centerX).toBe(10 * 512)
    expect(view.centerZ).toBe(1.5 * 512)
    expect(view.zoom).toBeCloseTo(1000 / (40 * 512))
  })
})

describe("blockAt", () => {
  it("floors negative coordinates", () => {
    const view: View = { centerX: 0, centerZ: 0, zoom: 4 }
    expect(blockAt(view, 499, 299, 1000, 600)).toEqual({ x: -1, z: -1 })
    expect(blockAt(view, 495, 300, 1000, 600)).toEqual({ x: -2, z: 0 })
    expect(blockAt(view, 504, 304, 1000, 600)).toEqual({ x: 1, z: 1 })
  })
})

describe("screenOf", () => {
  it("is where blockAt reads the same block", () => {
    const view: View = { centerX: -33.5, centerZ: 70, zoom: 3 }
    const at = screenOf(view, -40.5, 12.5, 1000, 600)
    expect(blockAt(view, at.left, at.top, 1000, 600)).toEqual({
      x: -41,
      z: 12
    })
  })
})
