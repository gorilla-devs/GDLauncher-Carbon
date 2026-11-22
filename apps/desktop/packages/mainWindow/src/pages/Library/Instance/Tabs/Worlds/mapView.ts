/** The geometry of a world's map on screen. Blocks are world coordinates
 *  (north is smaller z and up); `zoom` is screen pixels per block. */
export interface View {
  centerX: number
  centerZ: number
  zoom: number
}

/** A piece pixel smaller than this wastes the piece: the next scale up is
 *  as sharp on screen and a quarter of the pieces. */
export const MIN_PIXELS_PER_PIECE_PIXEL = 0.75
export const MAX_ZOOM = 8
/** Pixels on a side of every piece, at every scale. */
export const PIECE_PIXELS = 512
/** Pieces are loaded a few at a time, nearest the centre first, so this
 *  only bounds the work per view: enough for a large world zoomed out. */
const MAX_PIECES = 256
const MAX_SCALE = 4

/** The finest scale whose piece pixel is still at least
 *  `MIN_PIXELS_PER_PIECE_PIXEL` screen pixels. */
export function scaleFor(zoom: number): 0 | 1 | 2 | 3 | 4 {
  let scale = 0
  while (scale < MAX_SCALE && zoom * 2 ** scale < MIN_PIXELS_PER_PIECE_PIXEL)
    scale++
  return scale as 0 | 1 | 2 | 3 | 4
}

/** A piece's address and its box on screen. */
export interface Piece {
  scale: number
  x: number
  z: number
  left: number
  top: number
  size: number
}

/** The pieces that cover at least one existing region and touch the screen,
 *  nearest the centre first, at most `MAX_PIECES`. */
export function visiblePieces(
  view: View,
  width: number,
  height: number,
  regions: readonly { x: number; z: number }[]
): Piece[] {
  const scale = scaleFor(view.zoom)
  const regionsPerPiece = 2 ** scale
  const blocks = PIECE_PIXELS * regionsPerPiece
  const left = view.centerX - width / 2 / view.zoom
  const right = view.centerX + width / 2 / view.zoom
  const top = view.centerZ - height / 2 / view.zoom
  const bottom = view.centerZ + height / 2 / view.zoom

  const seen = new Set<string>()
  const pieces: (Piece & { distance: number })[] = []
  for (const region of regions) {
    const x = Math.floor(region.x / regionsPerPiece)
    const z = Math.floor(region.z / regionsPerPiece)
    const key = `${x},${z}`
    if (seen.has(key)) continue
    seen.add(key)
    const west = x * blocks
    const north = z * blocks
    if (west + blocks <= left || west >= right) continue
    if (north + blocks <= top || north >= bottom) continue
    const at = screenOf(view, west, north, width, height)
    pieces.push({
      scale,
      x,
      z,
      left: at.left,
      top: at.top,
      size: blocks * view.zoom,
      distance: Math.hypot(
        west + blocks / 2 - view.centerX,
        north + blocks / 2 - view.centerZ
      )
    })
  }
  pieces.sort((a, b) => a.distance - b.distance || a.z - b.z || a.x - b.x)
  return pieces
    .slice(0, MAX_PIECES)
    .map(({ distance: _distance, ...piece }) => piece)
}

/** A piece's box on screen in whole pixels. Each edge is rounded on its
 *  own, so neighbours share an edge exactly and no seam shows between them
 *  at any zoom. */
export function pieceBox(
  view: View,
  piece: { scale: number; x: number; z: number },
  width: number,
  height: number
): { left: number; top: number; width: number; height: number } {
  const blocks = PIECE_PIXELS * 2 ** piece.scale
  const near = screenOf(view, piece.x * blocks, piece.z * blocks, width, height)
  const far = screenOf(
    view,
    (piece.x + 1) * blocks,
    (piece.z + 1) * blocks,
    width,
    height
  )
  const left = Math.round(near.left)
  const top = Math.round(near.top)
  return {
    left,
    top,
    width: Math.round(far.left) - left,
    height: Math.round(far.top) - top
  }
}

/** Zooms by `factor` keeping the block under the screen point (px, pz)
 *  under it, the zoom held within `limits`. */
export function zoomAround(
  view: View,
  factor: number,
  px: number,
  pz: number,
  width: number,
  height: number,
  limits: { min: number; max: number }
): View {
  const zoom = Math.min(limits.max, Math.max(limits.min, view.zoom * factor))
  const dx = px - width / 2
  const dz = pz - height / 2
  return {
    centerX: view.centerX + dx / view.zoom - dx / zoom,
    centerZ: view.centerZ + dz / view.zoom - dz / zoom,
    zoom
  }
}

/** The view after the map was dragged by (dx, dy) screen pixels. */
export function dragged(view: View, dx: number, dy: number): View {
  return {
    centerX: view.centerX - dx / view.zoom,
    centerZ: view.centerZ - dy / view.zoom,
    zoom: view.zoom
  }
}

/** The view that shows every region, centred on them. */
export function fitAll(
  regions: readonly { x: number; z: number }[],
  width: number,
  height: number
): View {
  if (regions.length === 0 || width <= 0 || height <= 0)
    return { centerX: 0, centerZ: 0, zoom: 1 }
  let minX = Infinity
  let maxX = -Infinity
  let minZ = Infinity
  let maxZ = -Infinity
  for (const r of regions) {
    minX = Math.min(minX, r.x)
    maxX = Math.max(maxX, r.x)
    minZ = Math.min(minZ, r.z)
    maxZ = Math.max(maxZ, r.z)
  }
  const spanX = (maxX + 1 - minX) * PIECE_PIXELS
  const spanZ = (maxZ + 1 - minZ) * PIECE_PIXELS
  return {
    centerX: minX * PIECE_PIXELS + spanX / 2,
    centerZ: minZ * PIECE_PIXELS + spanZ / 2,
    zoom: Math.min(MAX_ZOOM, width / spanX, height / spanZ)
  }
}

/** The block under the screen point (px, py). */
export function blockAt(
  view: View,
  px: number,
  py: number,
  width: number,
  height: number
): { x: number; z: number } {
  return {
    x: Math.floor(view.centerX + (px - width / 2) / view.zoom),
    z: Math.floor(view.centerZ + (py - height / 2) / view.zoom)
  }
}

/** The screen point of the world point (x, z). */
export function screenOf(
  view: View,
  x: number,
  z: number,
  width: number,
  height: number
): { left: number; top: number } {
  return {
    left: (x - view.centerX) * view.zoom + width / 2,
    top: (z - view.centerZ) * view.zoom + height / 2
  }
}
