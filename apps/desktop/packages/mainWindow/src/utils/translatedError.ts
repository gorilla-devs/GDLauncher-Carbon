import type { NamespacedTranslationKey, TypedTFunction } from "@gd/i18n"

/** A refusal the player can act on. `key` names its text in the player's
 *  language; `message` stays English for logs. */
export class TranslatedError extends Error {
  constructor(
    readonly key: NamespacedTranslationKey,
    message: string
  ) {
    super(message)
    this.name = "TranslatedError"
  }
}

/** Toast text for a failed attempt: the translated reason of a
 *  TranslatedError, the backend's own sentence for any other error (shown
 *  as-is, like other backend errors, but never the raw rspc payload with its
 *  debug text and backtrace), or `fallbackKey` when there is no text. */
export function errorDescription(
  t: TypedTFunction,
  error: unknown,
  fallbackKey: NamespacedTranslationKey
): string {
  if (error instanceof TranslatedError) return t(error.key)
  const display =
    error instanceof Error ? extractErrorDisplay(error).trim() : ""
  return display || t(fallbackKey)
}
