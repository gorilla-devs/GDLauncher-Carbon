/** Byte transport stays exact until deliberately rounded for display. */
export type ByteCount = string | bigint | number
export function parseByteCount(value: ByteCount): bigint {
  if (typeof value === "bigint") {
    if (value < 0n) throw new Error("Byte count must be nonnegative")
    return value
  }
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value) || value < 0)
      throw new Error("Byte count must be a safe nonnegative integer")
    return BigInt(value)
  }
  if (!/^(0|[1-9][0-9]*)$/.test(value))
    throw new Error("Byte count must be a canonical decimal string")
  return BigInt(value)
}
export function bytePercent(current: ByteCount, total: ByteCount): number {
  const t = parseByteCount(total),
    c = parseByteCount(current)
  return t === 0n ? 0 : Number(((c > t ? t : c) * 10000n) / t) / 100
}
export function formatByteUnit(
  value: ByteCount,
  unit: bigint,
  decimals: number
): string {
  const scale = 10n ** BigInt(decimals)
  const rounded = (parseByteCount(value) * scale + unit / 2n) / unit
  return decimals === 0
    ? rounded.toString()
    : `${rounded / scale}.${(rounded % scale).toString().padStart(decimals, "0")}`
}
/** Binary units; existing safe numeric cache-size callers remain supported. */
export function formatBytes(bytes: ByteCount): string {
  if (typeof bytes === "number" && (!Number.isFinite(bytes) || bytes <= 0))
    return "0 B"
  const count = parseByteCount(bytes)
  if (count < 1024n) return `${count} B`
  if (count < 1048576n) return `${formatByteUnit(count, 1024n, 1)} KB`
  if (count < 1073741824n) return `${formatByteUnit(count, 1048576n, 1)} MB`
  return `${formatByteUnit(count, 1073741824n, 2)} GB`
}
