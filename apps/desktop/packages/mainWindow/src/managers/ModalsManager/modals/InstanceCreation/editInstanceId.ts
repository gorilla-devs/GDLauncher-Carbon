export function hasEditContext(data: { id?: unknown } | undefined): boolean {
  return data?.id !== undefined && data.id !== null
}

/** Match the complete supported numeric/string instance ID, never a prefix. */
export function editInstanceId(
  data: { id?: unknown } | undefined
): number | undefined {
  const raw = data?.id
  if (
    typeof raw !== "number" &&
    (typeof raw !== "string" || !/^\d+$/.test(raw))
  )
    return undefined
  const id = Number(raw)
  return Number.isSafeInteger(id) && id >= 0 && id <= 2147483647
    ? id
    : undefined
}
