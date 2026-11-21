import zlib from "node:zlib"

// Just enough PNG for the theme specs: writing small test images and reading
// Playwright's screenshots back as pixels (8-bit RGB or RGBA, not
// interlaced), with no dependency.

const SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])

export interface DecodedPng {
  width: number
  height: number
  /** RGBA, row after row. */
  data: Uint8Array
}

const chunk = (type: string, data: Buffer) => {
  const body = Buffer.concat([Buffer.from(type, "ascii"), data])
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(zlib.crc32(body) >>> 0)
  return Buffer.concat([len, body, crc])
}

const paeth = (a: number, b: number, c: number) => {
  const p = a + b - c
  const pa = Math.abs(p - a)
  const pb = Math.abs(p - b)
  const pc = Math.abs(p - c)
  if (pa <= pb && pa <= pc) return a
  return pb <= pc ? b : c
}

/**
 * Writes an 8-bit PNG. `filters` picks each row's filter (0 by default), so
 * the decoder can be tested against all five; `idatChunk` splits the data.
 */
export function encodePng(
  width: number,
  height: number,
  pixels: ArrayLike<number>,
  opts: { channels?: 3 | 4; filters?: number[]; idatChunk?: number } = {}
): Buffer {
  const bpp = opts.channels ?? 4
  const stride = width * bpp
  const raw = Buffer.alloc(height * (stride + 1))
  const at = (x: number, y: number) =>
    x < 0 || y < 0 ? 0 : pixels[y * stride + x]
  for (let y = 0; y < height; y++) {
    const filter = opts.filters?.[y] ?? 0
    raw[y * (stride + 1)] = filter
    for (let x = 0; x < stride; x++) {
      const v = pixels[y * stride + x]
      const a = at(x - bpp, y)
      const b = at(x, y - 1)
      const c = at(x - bpp, y - 1)
      const predicted = [0, a, b, Math.floor((a + b) / 2), paeth(a, b, c)][
        filter
      ]
      raw[y * (stride + 1) + 1 + x] = (v - predicted) & 0xff
    }
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8
  ihdr[9] = bpp === 4 ? 6 : 2
  const deflated = zlib.deflateSync(raw)
  const size = opts.idatChunk ?? deflated.length
  const idats: Buffer[] = []
  for (let i = 0; i < deflated.length; i += size) {
    idats.push(chunk("IDAT", deflated.subarray(i, i + size)))
  }
  return Buffer.concat([
    SIGNATURE,
    chunk("IHDR", ihdr),
    ...idats,
    chunk("IEND", Buffer.alloc(0))
  ])
}

/** A 1x1 PNG of one colour. */
export const solidPng = (r: number, g: number, b: number, a: number) =>
  encodePng(1, 1, [r, g, b, a])

export function decodePng(png: Buffer): DecodedPng {
  if (png.length < 8 || !png.subarray(0, 8).equals(SIGNATURE)) {
    throw new Error("not a PNG")
  }
  let pos = 8
  let width = 0
  let height = 0
  let channels = 0
  const idat: Buffer[] = []
  while (pos + 8 <= png.length) {
    const len = png.readUInt32BE(pos)
    const type = png.toString("ascii", pos + 4, pos + 8)
    const data = png.subarray(pos + 8, pos + 8 + len)
    if (type === "IHDR") {
      width = data.readUInt32BE(0)
      height = data.readUInt32BE(4)
      const [depth, colour, , , interlace] = data.subarray(8, 13)
      if (depth !== 8) throw new Error(`PNG bit depth ${depth} not supported`)
      if (colour !== 2 && colour !== 6) {
        throw new Error(`PNG colour type ${colour} not supported`)
      }
      if (interlace !== 0) throw new Error("interlaced PNG not supported")
      channels = colour === 6 ? 4 : 3
    } else if (type === "IDAT") {
      idat.push(data)
    } else if (type === "IEND") {
      break
    }
    pos += 12 + len
  }
  if (!channels) throw new Error("PNG has no IHDR")
  const raw = zlib.inflateSync(Buffer.concat(idat))
  const stride = width * channels
  const rows = new Uint8Array(height * stride)
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)]
    const src = y * (stride + 1) + 1
    const row = y * stride
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? rows[row + x - channels] : 0
      const b = y > 0 ? rows[row - stride + x] : 0
      const c = x >= channels && y > 0 ? rows[row - stride + x - channels] : 0
      let v = raw[src + x]
      if (filter === 1) v += a
      else if (filter === 2) v += b
      else if (filter === 3) v += Math.floor((a + b) / 2)
      else if (filter === 4) v += paeth(a, b, c)
      else if (filter !== 0) throw new Error(`PNG filter ${filter} unknown`)
      rows[row + x] = v & 0xff
    }
  }
  if (channels === 4) return { width, height, data: rows }
  const data = new Uint8Array(width * height * 4)
  for (let i = 0, j = 0; i < rows.length; i += 3, j += 4) {
    data[j] = rows[i]
    data[j + 1] = rows[i + 1]
    data[j + 2] = rows[i + 2]
    data[j + 3] = 255
  }
  return { width, height, data }
}

/**
 * How many pixels inside `rect` (clipped to the images) differ between two
 * shots of the same size: a pixel counts when any channel moved by more than
 * `tolerance`. It sees a faint blend (a picture behind a 95% dim moves the
 * backdrop by a few percent) that a colour match can't.
 */
export function countChangedPixels(
  a: DecodedPng,
  b: DecodedPng,
  rect: { x: number; y: number; width: number; height: number },
  tolerance = 3
): number {
  if (a.width !== b.width || a.height !== b.height) {
    throw new Error(
      `image size differs: ${a.width}x${a.height} vs ${b.width}x${b.height}`
    )
  }
  const x0 = Math.max(0, Math.floor(rect.x))
  const y0 = Math.max(0, Math.floor(rect.y))
  const x1 = Math.min(a.width, Math.ceil(rect.x + rect.width))
  const y1 = Math.min(a.height, Math.ceil(rect.y + rect.height))
  let n = 0
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const i = (y * a.width + x) * 4
      for (let c = 0; c < 4; c++) {
        if (Math.abs(a.data[i + c] - b.data[i + c]) > tolerance) {
          n++
          break
        }
      }
    }
  }
  return n
}

/** How many pixels inside `rect` (clipped to the image) match. */
export function countPixels(
  img: DecodedPng,
  rect: { x: number; y: number; width: number; height: number },
  match: (r: number, g: number, b: number, a: number) => boolean
): number {
  const x0 = Math.max(0, Math.floor(rect.x))
  const y0 = Math.max(0, Math.floor(rect.y))
  const x1 = Math.min(img.width, Math.ceil(rect.x + rect.width))
  const y1 = Math.min(img.height, Math.ceil(rect.y + rect.height))
  let n = 0
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const i = (y * img.width + x) * 4
      const d = img.data
      if (match(d[i], d[i + 1], d[i + 2], d[i + 3])) n++
    }
  }
  return n
}
