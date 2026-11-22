import { apiUrl } from "@/utils/rspcClient"

/** The address of one piece of a world's map. `rev` makes it a new address
 *  whenever the piece's regions change, so a kept answer is never stale. */
export function worldMapPieceUrl(p: {
  instanceId: number
  world: string
  dimension: string
  scale: number
  x: number
  z: number
  rev: string
}): string {
  const query = [
    ["instance_id", p.instanceId],
    ["world", p.world],
    ["dimension", p.dimension],
    ["scale", p.scale],
    ["x", p.x],
    ["z", p.z],
    ["rev", p.rev]
  ]
    .map(([key, value]) => `${key}=${encodeURIComponent(value)}`)
    .join("&")
  return apiUrl(`/instance/worldMapPiece?${query}`)
}
