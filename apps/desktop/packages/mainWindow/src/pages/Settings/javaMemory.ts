/** The least the launcher-wide maximum memory can be, in MB: where its
 * slider starts. A lower value never started a game. */
export const MIN_MEMORY_MB = 1024

/**
 * What a number typed in the memory box is saved as: whole MB, and at least
 * the slider's minimum, so 8 typed for 8 GB cannot become 8 MB. Anything that
 * is not a whole number gives null and leaves the setting as it is.
 */
export function typedMemoryMb(typed: string): number | null {
  const text = typed.trim()
  if (!/^\d{1,7}$/.test(text)) return null
  return Math.max(MIN_MEMORY_MB, Number(text))
}

/** The memory the page shows for what is saved: a value saved under the
 * minimum by an earlier version starts games with the minimum. */
export const shownMemoryMb = (saved: number | undefined) =>
  saved === undefined ? undefined : Math.max(MIN_MEMORY_MB, saved)
