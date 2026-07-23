import sanitizeHtml from "sanitize-html"

const opts = {
  allowedTags: sanitizeHtml.defaults.allowedTags.concat(["img", "iframe"]),
  allowedAttributes: {
    a: ["href", "name", "target", "class"],
    img: ["src", "width", "height", "class"],
    iframe: ["src", "width", "height", "allowfullscreen"]
  },
  allowedIframeHostnames: [
    // This is actually an official youtube domain lol https://who.is/whois/youtube-nocookie.com
    "www.youtube-nocookie.com",
    "www.youtube.com",
    "i.imgur.com",
    "cdn.ko-fi.com"
  ],
  transformTags: {
    a: sanitizeHtml.simpleTransform("a", { class: "text-blue-500" }),
    img: sanitizeHtml.simpleTransform("img", {
      class: "max-w-full h-auto"
    })
  }
}

export function parseToHtml(data: string | undefined) {
  return sanitizeHtml(data || "", opts)
}

// SVG icon markup (e.g. Modrinth category icons) is rendered inline via
// innerHTML. Strip anything scriptable while keeping the drawing tags and
// presentation attributes so the icon still renders (and inherits currentColor
// for theming). Tag/attribute casing is preserved for camelCase SVG tags like
// `linearGradient`/`viewBox`.
const svgIconOpts: sanitizeHtml.IOptions = {
  allowedTags: [
    "svg",
    "g",
    "path",
    "circle",
    "ellipse",
    "rect",
    "line",
    "polyline",
    "polygon",
    "defs",
    "linearGradient",
    "radialGradient",
    "stop",
    "clipPath",
    "mask",
    "use",
    "symbol",
    "title",
    "desc"
  ],
  allowedAttributes: {
    "*": [
      "d",
      "fill",
      "fill-rule",
      "fill-opacity",
      "stroke",
      "stroke-width",
      "stroke-linecap",
      "stroke-linejoin",
      "stroke-dasharray",
      "stroke-opacity",
      "clip-rule",
      "clip-path",
      "opacity",
      "transform",
      "viewBox",
      "width",
      "height",
      "x",
      "y",
      "x1",
      "y1",
      "x2",
      "y2",
      "cx",
      "cy",
      "r",
      "rx",
      "ry",
      "points",
      "offset",
      "stop-color",
      "stop-opacity",
      "gradientUnits",
      "gradientTransform",
      "xmlns",
      "class",
      "id"
    ]
  },
  parser: { lowerCaseTags: false, lowerCaseAttributeNames: false }
}

export function sanitizeSvgIcon(data: string | undefined) {
  return sanitizeHtml(data || "", svgIconOpts)
}
