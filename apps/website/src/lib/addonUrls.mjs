/** Recognize external HTTP(S) links, with deliberate bare DNS host support.
 * Relative site paths, credentials, unsupported schemes and malformed inputs
 * are withheld instead of being resolved against the launcher site. */
export function normalizeExternalWikiUrl(value) {
  if (typeof value !== "string") return null
  const input = value.trim()
  if (!input || /[\s\\\u0000-\u001f\u007f]/.test(input)) return null
  const absolute = /^https?:\/\//i.test(input)
  const bareHost = /^(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,63}(?::\d{1,5})?(?:[/?#].*)?$/i.test(input)
  if (!absolute && !bareHost) return null
  try {
    const url = new URL(absolute ? input : `https://${input}`)
    if (!['http:', 'https:'].includes(url.protocol) || !url.hostname || url.username || url.password) return null
    return url.href
  } catch { return null }
}
