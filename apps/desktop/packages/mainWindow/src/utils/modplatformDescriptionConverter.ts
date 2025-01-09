import { marked } from "marked"
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

export async function parseToHtml(
  data: string | undefined,
  type: "html" | "markdown"
) {
  if (type === "html") {
    return sanitizeHtml(data || "", opts)
  }

  // Sanitize html is needed to tranasform tags like a and img
  return sanitizeHtml(await marked.parse(data || ""), opts)
}
