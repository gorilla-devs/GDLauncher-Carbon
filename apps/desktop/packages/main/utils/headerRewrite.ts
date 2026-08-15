import type {
  BeforeSendResponse,
  HeadersReceivedResponse,
  OnBeforeSendHeadersListenerDetails,
  OnHeadersReceivedListenerDetails
} from "electron"

/** Case-insensitive insert-only header write: existing values always win. */
export function upsertKeyValue(
  obj: Record<string, unknown>,
  keyToChange: string,
  value: unknown
) {
  const keyToChangeLower = keyToChange.toLowerCase()
  for (const key of Object.keys(obj)) {
    if (key.toLowerCase() === keyToChangeLower) {
      return
    }
  }
  obj[keyToChange] = value
}

/**
 * Registered as the session's sole `onBeforeSendHeaders` listener (see the
 * comment at the registration site: Electron allows exactly one listener per
 * webRequest event per session, so all request-header rewriting lives here).
 *
 * YouTube's embedded player requires an HTTP Referer identifying the
 * embedding site; packaged builds load from file://, which sends none, so
 * the player refuses to play with error 153. Insert-only: when a real
 * Referer exists (dev server pages, the player's own sub-requests) it is
 * kept.
 *
 * `details.requestHeaders` is guarded against `undefined` before use, and
 * the whole body runs under try/catch, so `callback` is always invoked
 * exactly once — on any internal error the original (unmodified) headers
 * are passed through rather than leaving the request hanging.
 */
export function handleBeforeSendHeaders(
  details: OnBeforeSendHeadersListenerDetails,
  callback: (response: BeforeSendResponse) => void
): void {
  try {
    const requestHeaders = details.requestHeaders ?? {}

    let hostname = ""
    try {
      hostname = new URL(details.url).hostname
    } catch {
      // ignore unparseable URLs
    }
    if (
      hostname === "youtube.com" ||
      hostname.endsWith(".youtube.com") ||
      hostname === "youtube-nocookie.com" ||
      hostname.endsWith(".youtube-nocookie.com")
    ) {
      upsertKeyValue(requestHeaders, "Referer", "https://app.gdlauncher.com/")
    }

    callback({ requestHeaders })
  } catch (err) {
    console.error("[headers] onBeforeSendHeaders failed:", err)
    callback({})
  }
}

/**
 * Registered as the session's sole `onHeadersReceived` listener (see the
 * comment at the registration site: Electron allows exactly one listener per
 * webRequest event per session, so all response-header rewriting lives
 * here).
 *
 * The renderer runs from file:// in packaged builds, so cross-origin
 * responses need permissive CORS headers for fetches to succeed. Insert-only:
 * servers that set their own values keep them.
 *
 * `details.responseHeaders` is guarded against `undefined` before use, and
 * the whole body runs under try/catch, so `callback` is always invoked
 * exactly once — on any internal error the original (unmodified) headers
 * are passed through rather than leaving the request hanging.
 */
export function handleHeadersReceived(
  details: OnHeadersReceivedListenerDetails,
  callback: (response: HeadersReceivedResponse) => void
): void {
  try {
    const responseHeaders = details.responseHeaders ?? {}
    upsertKeyValue(responseHeaders, "Access-Control-Allow-Origin", ["*"])
    upsertKeyValue(responseHeaders, "Access-Control-Allow-Headers", ["*"])
    callback({ responseHeaders })
  } catch (err) {
    console.error("[headers] onHeadersReceived failed:", err)
    callback({})
  }
}
