import { describe, it, expect } from "vitest"
import { rewriteCssUrls } from "../cssUrls"

const resolve = (p: string) =>
  (
    ({
      "assets/x.png": "/x-hash.png",
      "assets/i.svg": "/i-hash.svg"
    }) as Record<string, string>
  )[p]

describe("rewriteCssUrls", () => {
  it("rewrites relative urls through the resolver", () => {
    const css =
      "a{background:url(assets/x.png)} b{background:url('assets/x.png')} c{mask:url(\"assets/i.svg#a\")}"
    expect(rewriteCssUrls(css, resolve)).toBe(
      'a{background:url("/x-hash.png")} b{background:url("/x-hash.png")} c{mask:url("/i-hash.svg#a")}'
    )
  })

  it("leaves fragments and unknown paths alone", () => {
    const css = "a{fill:url(#f)} b{background:url(assets/missing.png)}"
    expect(rewriteCssUrls(css, resolve)).toBe(css)
  })
})
