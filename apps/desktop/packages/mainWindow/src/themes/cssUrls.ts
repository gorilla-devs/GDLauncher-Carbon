// For trusted built-in CSS only; installed themes are rewritten by the core.
const URL_RE = /url\(\s*(?:"([^"]*)"|'([^']*)'|([^"'()\s]*))\s*\)/g

export function rewriteCssUrls(
  css: string,
  resolve: (relPath: string) => string | undefined
): string {
  return css.replace(
    URL_RE,
    (whole, dq?: string, sq?: string, bare?: string) => {
      const raw = dq ?? sq ?? bare ?? ""
      const hash = raw.indexOf("#")
      const path = hash === -1 ? raw : raw.slice(0, hash)
      const fragment = hash === -1 ? "" : raw.slice(hash)
      if (!path) return whole
      const resolved = resolve(path)
      return resolved === undefined ? whole : `url("${resolved}${fragment}")`
    }
  )
}
