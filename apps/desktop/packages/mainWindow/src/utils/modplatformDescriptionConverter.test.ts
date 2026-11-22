import { describe, expect, test } from "vitest"
import { parseToHtml } from "./modplatformDescriptionConverter"

// The packaged window is a local file, so this is what an address in a
// description is resolved against.
const PACKAGED_PAGE =
  "file:///C:/Program%20Files/GDLauncher/resources/app.asar/dist/mainWindow/index.html"

const addresses = (html: string, attribute: "src" | "href") =>
  [...html.matchAll(new RegExp(`\\b${attribute}="([^"]*)"`, "g"))].map(
    (match) => match[1].replace(/&amp;/g, "&")
  )

const requested = (html: string) =>
  addresses(html, "src").map((src) => new URL(src, PACKAGED_PAGE))

describe("an image in a description", () => {
  test.each([
    ["//attacker.example/share/a.png"],
    ["\\\\attacker.example\\share\\a.png"],
    ["/\\attacker.example/share/a.png"],
    ["\\/attacker.example/share/a.png"],
    [" //attacker.example/share/a.png"],
    ["/\t/attacker.example/share/a.png"],
    ["\n//attacker.example/share/a.png"],
    ["&#x2F;&#x2F;attacker.example/share/a.png"],
    ["&#92;&#92;attacker.example&#92;share&#92;a.png"]
  ])("that names only a host (%j) is asked over https", (src) => {
    const urls = requested(parseToHtml(`<img src="${src}">`))

    expect(urls.map((url) => url.href)).toEqual([
      "https://attacker.example/share/a.png"
    ])
  })

  test.each([
    ["../../x.png"],
    ["x.png"],
    ["/Windows/win.ini"],
    ["C:\\Windows\\win.ini"],
    ["file://attacker.example/share/a.png"],
    ["file:///C:/Windows/win.ini"],
    ["FILE://attacker.example/a.png"],
    ["smb://attacker.example/share/a.png"],
    ["javascript:alert(1)"],
    [""]
  ])("that is no web address (%j) asks for nothing", (src) => {
    const html = parseToHtml(`<img src="${src}" width="10">`)

    expect(addresses(html, "src")).toEqual([])
    expect(html).toContain("<img")
  })

  test.each([
    ["https://cdn.modrinth.com/data/AANobbMI/images/a.png"],
    ["https://media.forgecdn.net/attachments/1/2/a.png?x=1&y=2"],
    ["http://i.imgur.com/a.png"],
    ["HTTPS://cdn.modrinth.com/a.png"]
  ])("with a web address (%j) is kept", (src) => {
    const urls = requested(parseToHtml(`<img src="${src}">`))

    expect(urls).toHaveLength(1)
    expect(urls[0].href).toBe(new URL(src).href)
  })

  test("never reaches a file, whatever the address", () => {
    const hosts = ["//h/a", "\\\\h\\a", "/\\h/a", "\\/h/a", "///h/a", "////h/a"]
    const prefixes = ["", " ", "\t", "\n", "\r\n", "\u0000", "\u001f", "\f"]
    const html = parseToHtml(
      prefixes
        .flatMap((prefix) => hosts.map((host) => prefix + host))
        .concat(["file:/h/a", "file:\\\\h\\a", "fi\tle://h/a", "/\n\\h/a"])
        .map((src) => `<img src="${src}">`)
        .join("")
    )

    for (const url of requested(html)) expect(url.protocol).toBe("https:")
  })

  test("keeps its size and takes the class that fits it to the page", () => {
    expect(
      parseToHtml(
        '<img src="https://cdn.modrinth.com/a.png" width="40" height="20" onerror="x()">'
      )
    ).toBe(
      '<img src="https://cdn.modrinth.com/a.png" width="40" height="20" class="max-w-full h-auto" />'
    )
  })
})

describe("a link in a description", () => {
  test("that names only a host opens over https", () => {
    expect(
      addresses(parseToHtml('<a href="//modrinth.com/mod/x">x</a>'), "href")
    ).toEqual(["https://modrinth.com/mod/x"])
  })

  test.each([
    ["https://modrinth.com/mod/x?a=1&b=2"],
    ["mailto:someone@example.com"],
    ["#installation"]
  ])("with an ordinary address (%j) is left as written", (href) => {
    expect(addresses(parseToHtml(`<a href="${href}">x</a>`), "href")).toEqual([
      href
    ])
  })
})

describe("a video in a description", () => {
  test("from an allowed host that names only the host plays over https", () => {
    expect(
      addresses(
        parseToHtml('<iframe src="//www.youtube.com/embed/abc"></iframe>'),
        "src"
      )
    ).toEqual(["https://www.youtube.com/embed/abc"])
  })

  test("from any other host is not loaded", () => {
    expect(
      addresses(
        parseToHtml('<iframe src="//attacker.example/embed"></iframe>'),
        "src"
      )
    ).toEqual([])
  })
})
