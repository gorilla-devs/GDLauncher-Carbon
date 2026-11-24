import { fileName } from "../background"
import { spriteRasterSize } from "./particles"

// Loads each particle image once. A failed or empty load stays null (the
// particles skip it) and warns once, naming the file but not the token.

/** The long side of the first step a large raster source is drawn into. */
export const SPRITE_MAX_STEP = 2048

export type SpriteKind = "vector" | "raster"

/** An SVG (by its file name) is drawn at any size without loss. */
export const spriteKind = (url: string): SpriteKind =>
  fileName(url).toLowerCase().endsWith(".svg") ? "vector" : "raster"

/**
 * Draws a decoded image into a canvas of the raster size, always through a
 * canvas: createImageBitmap on an SVG with only a viewBox succeeds with a
 * transparent bitmap. A vector source is drawn straight at that size: it
 * gains nothing from steps, and an SVG may declare a huge width. A raster
 * source much larger than the target is halved in steps first, so the final
 * draw never skips over more than 2x of it and stays smooth (one jump from
 * 4000 px to 28 px aliases); its first step is at most SPRITE_MAX_STEP px on
 * the long side, so a huge picture can't ask for a huge canvas.
 */
export function drawSprite(
  source: CanvasImageSource,
  sourceWidth: number,
  sourceHeight: number,
  width: number,
  height: number,
  makeCanvas: () => HTMLCanvasElement,
  kind: SpriteKind = "raster"
): HTMLCanvasElement | null {
  let from: CanvasImageSource = source
  let fw = sourceWidth
  let fh = sourceHeight
  while (kind === "raster" && fw / 2 > width && fh / 2 > height) {
    let w = Math.ceil(fw / 2)
    let h = Math.ceil(fh / 2)
    const scale = Math.min(1, SPRITE_MAX_STEP / Math.max(w, h))
    w = Math.max(1, Math.round(w * scale))
    h = Math.max(1, Math.round(h * scale))
    const half = makeCanvas()
    half.width = w
    half.height = h
    const hctx = half.getContext("2d")
    if (!hctx) return null
    hctx.imageSmoothingQuality = "high"
    hctx.drawImage(from, 0, 0, half.width, half.height)
    from = half
    fw = half.width
    fh = half.height
  }
  const canvas = makeCanvas()
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext("2d")
  if (!ctx) return null
  ctx.imageSmoothingQuality = "high"
  ctx.drawImage(from, 0, 0, width, height)
  return canvas
}

/**
 * Decodes a theme image and rasterises it once at `spriteRasterSize` (as wide
 * as the largest drawn width, the image's aspect for the height), scaled
 * down at high quality. An animated image shows its first frame. The URL is
 * a theme asset URL the CSP already allows for <img>; nothing is read back.
 * No ImageBitmaps are kept, so there is nothing to close.
 */
export async function rasterizeSprite(
  url: string,
  sizes: { minSize?: number | null; maxSize?: number | null }
): Promise<CanvasImageSource | null> {
  const img = new Image()
  img.src = url
  await img.decode()
  const { naturalWidth: w, naturalHeight: h } = img
  if (!w || !h) return null
  const { width, height } = spriteRasterSize(sizes, w, h)
  return drawSprite(
    img,
    w,
    h,
    width,
    height,
    () => document.createElement("canvas"),
    spriteKind(url)
  )
}

export function createSpriteCache(
  load: (url: string) => Promise<CanvasImageSource | null>
): {
  get(url: string): CanvasImageSource | null
  ready(url: string): Promise<void>
} {
  const entries = new Map<
    string,
    { value: CanvasImageSource | null; done: Promise<void> }
  >()

  const start = (url: string) => {
    let entry = entries.get(url)
    if (entry) return entry
    const fresh: { value: CanvasImageSource | null; done: Promise<void> } = {
      value: null,
      done: Promise.resolve()
    }
    const warn = () =>
      console.warn(`Theme particle image failed to load: ${fileName(url)}`)
    let pending: Promise<CanvasImageSource | null>
    try {
      pending = load(url)
    } catch (e) {
      pending = Promise.reject(e instanceof Error ? e : new Error(String(e)))
    }
    fresh.done = pending.then(
      (value) => {
        fresh.value = value ?? null
        if (!value) warn()
      },
      () => warn()
    )
    entries.set(url, fresh)
    entry = fresh
    return entry
  }

  return {
    get: (url) => start(url).value,
    ready: (url) => start(url).done
  }
}
