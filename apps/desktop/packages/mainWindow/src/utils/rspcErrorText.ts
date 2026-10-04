// ---------------------------------------------------------------------------
// rspc error parsing
//
// rspc wraps errors as {code, message} where message is a JSON string
// containing {cause, backtrace}. `parseRspcErrorPayload` is the one place
// that shape gets parsed; the global toast handler in rspcClient.ts and any
// UI that needs the same clean text (via `extractErrorDisplay`) route
// through it so they can never drift out of sync with each other. This
// module imports nothing, so plain helpers can use it without loading the
// rspc client.
// ---------------------------------------------------------------------------

export interface RspcErrorCause {
  display: string
  code?: string
  data?: unknown
}

export interface ParsedRspcError {
  code?: number
  message?: string
  cause?: RspcErrorCause[]
  /** Set only when the payload parsed to valid JSON that isn't the expected
   *  `{cause}`/`{message}` shape (e.g. the bare literal `"null"`, a number,
   *  a string). Carries the untouched raw text so callers still have
   *  something to show instead of crashing on the missing fields. */
  display?: string
}

export function parseRspcErrorPayload(
  raw: string
): ParsedRspcError | undefined {
  let parsed: ParsedRspcError
  try {
    parsed = JSON.parse(raw) as ParsedRspcError
  } catch {
    return undefined
  }

  if (parsed == null || typeof parsed !== "object") {
    return { display: raw }
  }

  // rspc wraps errors as {code, message} where message is a JSON string containing {cause, backtrace}
  if (parsed.message && !parsed.cause) {
    try {
      const inner = JSON.parse(parsed.message) as { cause?: RspcErrorCause[] }
      if (inner.cause) {
        parsed = inner
      }
    } catch {
      // message wasn't JSON, keep parsed as-is
    }
  }

  return parsed
}

/** Extracts a short, human-readable message from an rspc error (or any
 *  thrown value): the first cause's `display` when the rspc {cause,
 *  backtrace} payload parses, else the parsed payload's own `message`, else
 *  its raw fallback `display` text, else the error's `message`, else its
 *  stringified form. Callers that render an error inline (rather than
 *  relying on the global toast in rspcClient.ts) should use this instead of
 *  `error.message` — the raw rspc error message can carry a full backend
 *  backtrace. */
export function extractErrorDisplay(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error)
  const parsed = parseRspcErrorPayload(message)
  return (
    parsed?.cause?.[0]?.display ?? parsed?.message ?? parsed?.display ?? message
  )
}

/** The first cause of an rspc error that carries a machine-readable `code`
 *  (for example SERVER_RECOVERED_PROCESS), or undefined. */
export function extractErrorCodeCause(
  error: unknown
): RspcErrorCause | undefined {
  const message = error instanceof Error ? error.message : String(error)
  return parseRspcErrorPayload(message)?.cause?.find((c) => c.code)
}

/** The coded causes the global handler in rspcClient.ts presents itself, as
 *  a translated toast or a dialog, each with what it needs to do so. That
 *  handler presents a coded cause only through this table (its presenters
 *  are typed by these keys), so a mutation's own failure toast, which stays
 *  quiet for exactly these, never repeats it or leaves it unshown. */
export const GLOBALLY_PRESENTED = {
  ACCOUNT_BANNED: () => true,
  SERVER_RECOVERED_PROCESS: () => true,
  INSUFFICIENT_MEMORY: (cause: RspcErrorCause) => !!cause.data,
  EULA_NOT_ACCEPTED: (cause: RspcErrorCause) => !!cause.data
}

export type GloballyPresentedCode = keyof typeof GLOBALLY_PRESENTED

/** Whether the global handler in rspcClient.ts presents this coded cause
 *  itself, so a mutation's own failure toast would only repeat it. */
export function presentedGlobally(
  cause: RspcErrorCause | undefined
): cause is RspcErrorCause & { code: GloballyPresentedCode } {
  if (
    !cause?.code ||
    !Object.prototype.hasOwnProperty.call(GLOBALLY_PRESENTED, cause.code)
  ) {
    return false
  }
  return GLOBALLY_PRESENTED[cause.code as GloballyPresentedCode](cause)
}
