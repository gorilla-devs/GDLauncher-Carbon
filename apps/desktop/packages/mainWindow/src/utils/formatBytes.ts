/**
 * Format a byte count into a short human-readable string.
 * Uses binary (1024-based) units to match SQLite / file system conventions.
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B"
  if (bytes < 1024) return `${bytes.toFixed(0)} B`

  const kb = bytes / 1024
  if (kb < 1024) return `${kb.toFixed(1)} KB`

  const mb = kb / 1024
  if (mb < 1024) return `${mb.toFixed(1)} MB`

  const gb = mb / 1024
  return `${gb.toFixed(2)} GB`
}
