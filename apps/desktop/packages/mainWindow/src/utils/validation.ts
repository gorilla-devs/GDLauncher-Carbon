import { z } from "zod"

export const MAX_DOWNLOADS_LIMIT = 10_000_000

/**
 * Schema for max downloads input - validates and transforms to a clamped number
 */
export const maxDownloadsSchema = z
  .string()
  .transform((val) => {
    const trimmed = val.trim()
    if (trimmed === "") return null
    const num = parseInt(trimmed, 10)
    if (isNaN(num)) return null
    return Math.min(Math.max(num, 1), MAX_DOWNLOADS_LIMIT)
  })
  .nullable()

/**
 * Validates and clamps a max downloads input value.
 * Returns the clamped string value, or empty string if input is empty/invalid.
 */
export function validateMaxDownloads(value: string): string {
  const trimmed = value.trim()
  if (trimmed === "") return ""

  const num = parseInt(trimmed, 10)
  if (isNaN(num)) return ""

  if (num > MAX_DOWNLOADS_LIMIT) return MAX_DOWNLOADS_LIMIT.toString()
  if (num < 1) return "1"

  return trimmed
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
