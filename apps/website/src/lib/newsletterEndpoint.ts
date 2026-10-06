/** SSR-only configuration boundary. Only this public URL crosses the island. */
export function resolveNewsletterEndpoint(base: unknown): string {
  if (typeof base !== "string" || !base.trim()) {
    throw new Error("ENDERIUM_API_BASE is not set. Add it to .env or the deploy environment.")
  }
  try {
    const url = new URL(base.trim())
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) throw new Error()
    url.pathname = `${url.pathname.replace(/\/+$/, "")}/mailing`
    return url.href
  } catch {
    throw new Error("ENDERIUM_API_BASE must be an absolute HTTP(S) URL without credentials, query or fragment.")
  }
}
