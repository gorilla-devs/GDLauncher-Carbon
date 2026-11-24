// The logo pattern drawn behind the ad column (ThemedPatternSVG). Plain DOM
// code so it can be tested on the real asset.

const BACKGROUND_FILL = "rgb(var(--pattern-background))"
const LOGO_FILL = "rgb(var(--pattern-fill))"

const number = (value: string | null) =>
  value === null ? NaN : Number.parseFloat(value)

/**
 * Parses the pattern SVG and readies it for the column:
 * - a rect that covers the whole picture in the background colour is left
 *   unpainted, so the column's own colour, which a theme's
 *   `ad-column-background` rule sets, shows through. However the asset is
 *   exported, no painted full-size rect stays;
 * - the logos get `pattern-fill` (their hooked layer's colour) and the marks
 *   cut into them `pattern-bg`;
 * - the picture fills its box, cropped rather than letterboxed.
 */
export function themedPatternSvg(raw: string): SVGSVGElement | null {
  const doc = new DOMParser().parseFromString(raw, "image/svg+xml")
  const svg = doc.querySelector("svg")
  if (!svg) return null

  const box = (svg.getAttribute("viewBox") ?? "").split(/[\s,]+/).map(Number)
  const width = box.length === 4 ? box[2] : number(svg.getAttribute("width"))
  const height = box.length === 4 ? box[3] : number(svg.getAttribute("height"))
  const covers = (el: Element) =>
    el.tagName.toLowerCase() === "rect" &&
    !el.closest("clipPath, mask, defs") &&
    (number(el.getAttribute("x")) || 0) <= 0 &&
    (number(el.getAttribute("y")) || 0) <= 0 &&
    (el.getAttribute("width") === "100%" ||
      number(el.getAttribute("width")) >= width) &&
    (el.getAttribute("height") === "100%" ||
      number(el.getAttribute("height")) >= height)

  for (const el of svg.querySelectorAll(`[fill="${BACKGROUND_FILL}"]`)) {
    if (covers(el)) {
      el.setAttribute("fill", "none")
    } else {
      el.removeAttribute("fill")
      el.setAttribute("class", "pattern-bg")
    }
  }
  for (const el of svg.querySelectorAll(`[fill="${LOGO_FILL}"]`)) {
    el.removeAttribute("fill")
    el.setAttribute("class", "pattern-fill")
  }

  svg.removeAttribute("width")
  svg.removeAttribute("height")
  svg.setAttribute("style", "width: 100%; height: 100%;")
  svg.setAttribute("preserveAspectRatio", "xMidYMid slice")
  return svg as unknown as SVGSVGElement
}
