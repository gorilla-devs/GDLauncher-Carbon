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
