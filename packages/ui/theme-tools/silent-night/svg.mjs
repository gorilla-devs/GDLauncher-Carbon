const escapeAttr = (v) =>
  String(v)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")

/**
 * Builds one element as a string. `children` is a string or an array of
 * strings; attributes whose value is null or undefined are skipped.
 */
export function el(tag, attrs = {}, children) {
  let out = "<" + tag
  for (const k of Object.keys(attrs)) {
    const v = attrs[k]
    if (v == null) continue
    out += ` ${k}="${escapeAttr(v)}"`
  }
  const inner = Array.isArray(children) ? children.join("") : children
  return inner ? `${out}>${inner}</${tag}>` : `${out}/>`
}

/** The mock's rounding to one decimal. */
export function f(n) {
  return Math.round(n * 10) / 10
}

export const SVG_NS = "http://www.w3.org/2000/svg"

/** Wraps parts in an <svg> document with the namespace first. */
export function svgDoc(attrs, children) {
  return el("svg", { xmlns: SVG_NS, ...attrs }, children) + "\n"
}
