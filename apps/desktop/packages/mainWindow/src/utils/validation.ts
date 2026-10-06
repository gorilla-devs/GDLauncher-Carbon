import { z } from "zod"

export const MAX_DOWNLOADS_LIMIT = 10_000_000

/** Interpret a complete decimal number input, preserving exponent notation. */
export function parseMaxDownloads(value: string | null): number | null {
  const trimmed = value?.trim() ?? ""
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(trimmed)) {
    return null
  }
  const number = Number(trimmed)
  if (!Number.isFinite(number) || !Number.isInteger(number)) return null
  return Math.min(Math.max(number, 1), MAX_DOWNLOADS_LIMIT)
}

/**
 * Schema for max downloads input - validates and transforms to a clamped number
 */
export const maxDownloadsSchema = z
  .string()
  .transform(parseMaxDownloads)
  .nullable()

/**
 * Validates and clamps a max downloads input value.
 * Returns the clamped string value, or empty string if input is empty/invalid.
 */
export function validateMaxDownloads(value: string): string {
  return parseMaxDownloads(value)?.toString() ?? ""
}

/**
 * Schema for share instance form
 */
export const shareInstanceFormSchema = z.object({
  title: z.string().max(100).optional(),
  expirationDays: z.enum(["1", "7", "30", "90"]),
  maxDownloads: maxDownloadsSchema
})

export type ShareInstanceForm = z.infer<typeof shareInstanceFormSchema>
