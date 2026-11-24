// @vitest-environment jsdom
import { readFileSync } from "fs"
import { join } from "path"
import { afterEach, describe, expect, it, vi } from "vitest"

const html = () => readFileSync(join(__dirname, "../page/index.html"), "utf8")

function load(query: string) {
  const calls = { why: 0, remove: 0 }
  let cb: ((s: { whyAds: string; removeAds: string | null }) => void) | null =
    null
  history.replaceState({}, "", `/adview/index.html?${query}`)
  ;(window as any).gdlAdView = {
    openWhyAds: vi.fn(() => calls.why++),
    openRemoveAds: vi.fn(() => calls.remove++),
    onState: (f: typeof cb) => {
      cb = f
    }
  }
  const page = html()
  const script = /<script>([\s\S]*?)<\/script>/.exec(page)![1]
  document.documentElement.innerHTML = page.replace(
    /<script>[\s\S]*?<\/script>/,
    ""
  )
  // eslint-disable-next-line @typescript-eslint/no-implied-eval -- runs the real page script under test
  new Function(script)()
  return { doc: document, calls, push: (s: any) => cb!(s) }
}

afterEach(() => {
  document.documentElement.innerHTML = "<head></head><body></body>"
  delete (window as any).gdlAdView
})

const FULL = "w=440&h=730&bw=400&bh=60&banner=1&hideText=0"

describe("ad page", () => {
  it("builds the banner and the high-impact main slot", () => {
    const { doc } = load(FULL)
    const ads = doc.querySelectorAll("owadview")
    expect(ads.length).toBe(2)
    expect(ads[0].getAttribute("cid")).toBe("gdlauncher_horizontal_400_60")
    expect(ads[1].getAttribute("cid")).toBe(
      "gdlauncher_rectangle_440_730_high_impact"
    )
    expect(ads[1].getAttribute("adstyle")).toBe("high-impact-ad")
    const a = ads[0].parentElement!.style
    const b = ads[1].parentElement!.style
    expect([a.width, a.height]).toEqual(["400px", "60px"])
    expect([b.width, b.height]).toEqual(["440px", "730px"])
  })

  it("placeholder mode draws boxes, never ads", () => {
    const { doc } = load(FULL + "&placeholder=1")
    expect(doc.querySelectorAll("owadview").length).toBe(0)
    const boxes = [
      ...doc.querySelectorAll<HTMLElement>("[data-ad-placeholder]")
    ]
    expect(boxes.length).toBe(2)
    expect(boxes.map((b) => [b.style.width, b.style.height])).toEqual([
      ["400px", "60px"],
      ["440px", "730px"]
    ])
    expect(boxes[0].textContent).toBe("AD")
    expect(doc.getElementById("why")).not.toBeNull()
  })

  it("shows state labels and the remove-ads link", () => {
    const { doc, push } = load(FULL)
    const why = doc.getElementById("why")!
    const remove = doc.getElementById("remove-ads")!
    expect(why.textContent).toBe("Why are ads needed?")
    expect(remove.hasAttribute("hidden")).toBe(true)
    push({ whyAds: "Perché?", removeAds: null })
    expect(why.textContent).toBe("Perché?")
    expect(remove.hasAttribute("hidden")).toBe(true)
    push({ whyAds: "Perché?", removeAds: "Remove ads" })
    expect(remove.hasAttribute("hidden")).toBe(false)
    expect(remove.textContent).toBe("Remove ads")
  })

  it("forwards clicks to the preload api", () => {
    const { doc, calls } = load(FULL)
    doc.getElementById("why")!.click()
    expect(calls.why).toBe(1)
    doc.getElementById("remove-ads")!.click()
    expect(calls.remove).toBe(1)
  })

  it("puts both links in one row", () => {
    const { doc } = load(FULL)
    const links = doc.getElementById("links")!
    expect(links.contains(doc.getElementById("why"))).toBe(true)
    expect(links.contains(doc.getElementById("remove-ads"))).toBe(true)
  })

  it("honours hideText and the no-banner layout", () => {
    expect(
      load("w=440&h=730&bw=400&bh=60&banner=1&hideText=1").doc.getElementById(
        "why"
      )
    ).toBeNull()
    const hidden = load("w=440&h=730&bw=400&bh=60&banner=1&hideText=1").doc
    expect(hidden.getElementById("links")).toBeNull()
    expect(hidden.getElementById("remove-ads")).toBeNull()
    const { doc } = load("w=160&h=600&bw=0&bh=0&banner=0&hideText=0")
    const ads = doc.querySelectorAll("owadview")
    expect(ads.length).toBe(1)
    expect(ads[0].getAttribute("cid")).toBe("gdlauncher_rectangle_160_600")
    expect(ads[0].hasAttribute("adstyle")).toBe(false)
  })
})
