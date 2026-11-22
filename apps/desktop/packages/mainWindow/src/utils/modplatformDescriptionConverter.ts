import sanitizeHtml from "sanitize-html"

// An address as the browser reads it: nothing up to a space around it, and no
// tab or line break anywhere in it.
const written = (value: string | undefined) => {
  const text = value ?? ""
  let start = 0
  let end = text.length
  while (start < end && text.charCodeAt(start) <= 0x20) start++
  while (end > start && text.charCodeAt(end - 1) <= 0x20) end--
  return text.slice(start, end).replace(/[\t\n\r]/g, "")
}

// An address that says by itself where it leads, with no page to complete it.
const complete = (address: string) => {
  try {
    return new URL(address)
  } catch {
    return undefined
  }
}

// An address that names a host and no scheme (`//host/a.png`, with either
// slash) takes its page's scheme. The packaged window is a local file, where
// that makes it a file on another computer, and Windows signs in to that
// computer to fetch it. It is read as https, as on the site it was written for.
const hostOnly = (address: string) =>
  /^[/\\]{2}/.test(address) ? complete(`https:${address}`) : undefined

// The address a description may make the window load by itself: a web one.
// Any other (a path, a drive, a file) would be completed from the window's own
// place on disk, so it loads nothing.
const loaded = (value: string | undefined) => {
  const address = written(value)
  const url = complete(address) ?? hostOnly(address)
  return url?.protocol === "https:" || url?.protocol === "http:"
    ? url.href
    : undefined
}

// A link opens only when it is clicked, and the main process opens web and
// mail addresses alone: it is left as written, a host-only one made https.
const opened = (value: string) => {
  const address = written(value)
  return complete(address) ? value : (hostOnly(address)?.href ?? value)
}

const withSource =
  (extra: Record<string, string>): sanitizeHtml.Transformer =>
  (tagName, attribs) => {
    const { src, ...rest } = attribs
    const address = loaded(src)
    return {
      tagName,
      attribs: address
        ? { ...attribs, src: address, ...extra }
        : { ...rest, ...extra }
    }
  }

const opts: sanitizeHtml.IOptions = {
  allowedTags: sanitizeHtml.defaults.allowedTags.concat(["img", "iframe"]),
  allowProtocolRelative: false,
  allowedSchemesByTag: { img: ["https", "http"], iframe: ["https"] },
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
    a: (tagName, attribs) => ({
      tagName,
      attribs: {
        ...attribs,
        ...(attribs.href !== undefined && { href: opened(attribs.href) }),
        class: "text-blue-500"
      }
    }),
    img: withSource({ class: "max-w-full h-auto" }),
    iframe: withSource({})
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
