import { describe, expect, it } from "vitest"
import {
  countChangedPixels,
  countPixels,
  decodePng,
  encodePng,
  solidPng
} from "./png.js"

// A 5x5 image whose every channel differs from its neighbours, so each
// filter's left, up and up-left terms all matter.
const W = 5
const H = 5
const pixels = (channels: 3 | 4) => {
  const out: number[] = []
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      out.push(
        (x * 53 + y * 17) % 256,
        (x * 11 + y * 97) % 256,
        (x * y * 29 + 7) % 256
      )
      if (channels === 4) out.push((200 + x * 13 + y * 3) % 256)
    }
  }
  return out
}
const rgba = (rgb: number[]) => {
  const out: number[] = []
  for (let i = 0; i < rgb.length; i += 3)
    out.push(rgb[i], rgb[i + 1], rgb[i + 2], 255)
  return out
}

describe("decodePng", () => {
  it("reads a solid 1x1 pixel", () => {
    const img = decodePng(solidPng(255, 0, 255, 255))
    expect(img.width).toBe(1)
    expect(img.height).toBe(1)
    expect([...img.data]).toEqual([255, 0, 255, 255])
  })

  it("undoes every row filter, RGBA", () => {
    const src = pixels(4)
    const png = encodePng(W, H, src, { filters: [0, 1, 2, 3, 4] })
    const img = decodePng(png)
    expect([img.width, img.height]).toEqual([W, H])
    expect([...img.data]).toEqual(src)
  })

  it("undoes every row filter, RGB, and returns RGBA", () => {
    const src = pixels(3)
    const png = encodePng(W, H, src, { channels: 3, filters: [4, 3, 2, 1, 0] })
    expect([...decodePng(png).data]).toEqual(rgba(src))
  })

  it("joins split IDAT chunks", () => {
    const src = pixels(4)
    const png = encodePng(W, H, src, { filters: [4, 4, 4, 4, 4], idatChunk: 7 })
    expect([...decodePng(png).data]).toEqual(src)
  })

  it("refuses what it can't read", () => {
    expect(() => decodePng(Buffer.from("not a png"))).toThrow(/not a PNG/)
    const palette = Buffer.from(solidPng(0, 0, 0, 0))
    palette[25] = 3 // colour type byte of IHDR
    expect(() => decodePng(palette)).toThrow(/colour type 3/)
  })
})

describe("countPixels", () => {
  it("counts matches inside the rect only, clipped to the image", () => {
    const src: number[] = []
    for (let i = 0; i < W * H; i++) src.push(255, 0, 255, 255)
    const img = decodePng(encodePng(W, H, src))
    const magenta = (r: number, g: number, b: number) =>
      r === 255 && g === 0 && b === 255
    expect(countPixels(img, { x: 1, y: 1, width: 2, height: 3 }, magenta)).toBe(
      6
    )
    expect(
      countPixels(img, { x: 3, y: 3, width: 10, height: 10 }, magenta)
    ).toBe(4)
    expect(countPixels(img, { x: 0, y: 0, width: 0, height: 5 }, magenta)).toBe(
      0
    )
  })
})

describe("countChangedPixels", () => {
  const dark = [24, 28, 40, 255]
  const image = (at: Record<number, number[]> = {}) => {
    const src: number[] = []
    for (let i = 0; i < W * H; i++) src.push(...(at[i] ?? dark))
    return decodePng(encodePng(W, H, src))
  }
  const all = { x: 0, y: 0, width: W, height: H }

  it("sees a 5% blend of magenta that a colour match misses", () => {
    // #ff00ff at 5% over the dark backdrop
    const blend = dark.map((c, i) =>
      i === 3 ? 255 : Math.round(c * 0.95 + [255, 0, 255][i] * 0.05)
    )
    const after = image({ 7: blend, 18: blend })
    const magenta = (r: number, g: number, b: number) =>
      r > 200 && b > 200 && g < 90
    expect(countPixels(after, all, magenta)).toBe(0)
    expect(countChangedPixels(image(), after, all)).toBe(2)
  })

  it("counts only inside the rect, clipped to the image", () => {
    const after = image({ 0: [255, 255, 255, 255], 24: [255, 255, 255, 255] })
    expect(
      countChangedPixels(image(), after, { x: 1, y: 1, width: 10, height: 10 })
    ).toBe(1)
    expect(
      countChangedPixels(image(), after, { x: 0, y: 0, width: 0, height: 5 })
    ).toBe(0)
  })

  it("ignores a difference within the tolerance", () => {
    const near = image({ 3: [26, 27, 42, 255] })
    expect(countChangedPixels(image(), near, all)).toBe(0)
    expect(countChangedPixels(image(), near, all, 1)).toBe(1)
  })

  it("refuses images of different sizes", () => {
    const small = decodePng(solidPng(0, 0, 0, 255))
    expect(() => countChangedPixels(image(), small, all)).toThrow(/size/)
  })
})
