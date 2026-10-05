export interface InvalidateOperation {
  key: string
  args: unknown
}
export function parseInvalidateFrame(
  raw: unknown
): InvalidateOperation | undefined {
  try {
    return JSON.parse(raw as string) as InvalidateOperation
  } catch (error) {
    console.warn("Invalidations channel: dropping unparsable frame", error)
    return undefined
  }
}
export function invalidationKey(data: InvalidateOperation): unknown[] {
  const key: unknown[] = [data.key]
  if (data.args !== null) key.push(data.args)
  return key
}
