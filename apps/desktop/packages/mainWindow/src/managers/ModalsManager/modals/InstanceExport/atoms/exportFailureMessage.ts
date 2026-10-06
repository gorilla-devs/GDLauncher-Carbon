export function exportFailureMessage(
  causes: readonly { display: string }[] | undefined,
  generic: string
): string {
  return causes?.[1]?.display.trim() || causes?.[0]?.display.trim() || generic
}
