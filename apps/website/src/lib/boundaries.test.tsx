import { afterEach, describe, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import fs from "node:fs"
import vm from "node:vm"
import { transpileModule } from "typescript"
import Newsletter from "../components/Newsletter"
import { normalizeExternalWikiUrl } from "./addonUrls.mjs"
import { resolveNewsletterEndpoint } from "./newsletterEndpoint"
import { extractCurseForge, extractModrinthProject, extractModrinthSearch } from "../../scripts/update-addons.mjs"

let dispose = () => {}
afterEach(() => { dispose(); document.body.innerHTML = ""; document.body.style.overflow = ""; vi.unstubAllGlobals() })
const project = { id: "a", title: "Example", slug: "fixture", project_type: "mod", source_url: "https://github.com/example/source" }

describe("addon links", () => {
  it("keeps Modrinth CTA and repository separate", () => {
    const entry = extractModrinthProject(project, "modpack")
    expect(entry.websiteUrl).toBe("https://modrinth.com/modpack/fixture")
    expect(entry.sourceUrl).toBe(project.source_url)
  })
  it("retains no-source, search and datapack intent controls", () => {
    expect(extractModrinthProject({ ...project, source_url: null }).websiteUrl).toBe("https://modrinth.com/mod/fixture")
    expect(extractModrinthProject(project, "datapack").websiteUrl).toBe("https://modrinth.com/datapack/fixture")
    expect(extractModrinthSearch({ ...project, project_id: "a" }, "datapack").websiteUrl).toBe("https://modrinth.com/datapack/fixture")
  })
  it.each([
    ["craftingdead.net", "https://craftingdead.net/"],
    ["Twitch.tv/MiniiiGod", "https://twitch.tv/MiniiiGod"],
    ["akashicrecords.gg", "https://akashicrecords.gg/"],
    ["beyondmines.net/", "https://beyondmines.net/"],
    ["https://wiki.example/docs?q=a#one", "https://wiki.example/docs?q=a#one"],
    ["http://wiki.example/wiki", "http://wiki.example/wiki"],
  ])("normalizes deliberate external input %s", (input, expected) => {
    expect(normalizeExternalWikiUrl(input)).toBe(expected)
    expect(extractCurseForge({ name: "A", links: { websiteUrl: "https://www.curseforge.com/minecraft/mods/a", wikiUrl: input } }).wikiUrl).toBe(expected)
    expect(extractModrinthProject({ ...project, wiki_url: input }).wikiUrl).toBe(expected)
  })
  it.each([null, "", "docs/wiki", "/wiki", "//wiki.example", "javascript:alert(1)", "ftp://wiki.example", "https://user:pass@wiki.example", "https://", "wiki.example\\path", "wiki.example/has space"])("withholds invalid Wiki %s", input => {
    expect(normalizeExternalWikiUrl(input)).toBeNull()
  })
})

const labels = { heading: "Newsletter", emailPlaceholder: "Email", subscribe: "Subscribe", errorAlreadySubscribed: "Already", errorMissingInfo: "Missing", errorInvalidEmail: "Invalid", errorGeneric: "Failed", success: "Subscribed" }
function mount() {
  const host = document.createElement("div"); document.body.append(host)
  dispose = render(() => <Newsletter labels={labels} endpoint="https://newsletter.example.invalid/v1/mailing" />, host)
  const input = host.querySelector("input")!
  const submit = (value: string) => { input.value = value; input.dispatchEvent(new Event("input", { bubbles: true })); host.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })) }
  return { host, input, submit }
}
const settle = () => new Promise(resolve => setTimeout(resolve, 0))
describe("newsletter island boundary", () => {
  it("validates SSR configuration and appends mailing once", () => {
    expect(resolveNewsletterEndpoint("https://mail.example.invalid/v1/")).toBe("https://mail.example.invalid/v1/mailing")
    for (const value of [undefined, "", "relative", "ftp://mail.example", "https://user:pass@mail.example", "https://mail.example?q=a", "https://mail.example#one"]) expect(() => resolveNewsletterEndpoint(value)).toThrow()
  })
  it("submits trimmed email using only the supplied endpoint, blocks duplicates, clears on success", async () => {
    let finish!: (res: unknown) => void
    const transport = vi.fn(() => new Promise(resolve => { finish = resolve }))
    vi.stubGlobal("fetch", transport)
    const { host, input, submit } = mount()
    submit("  reader@example.museum  "); submit("reader@example.museum")
    expect(transport).toHaveBeenCalledTimes(1)
    expect(transport.mock.calls[0]).toEqual(["https://newsletter.example.invalid/v1/mailing", { method: "POST", headers: { Accept: "application/json", "Content-Type": "application/json" }, body: '{"email":"reader@example.museum"}' }])
    expect(host.querySelector("button")!.disabled).toBe(true)
    finish({ ok: true }); await settle()
    expect(input.value).toBe("")
    expect(host.querySelector('[role="status"]')!.textContent).toBe("Subscribed")
    expect(host.querySelector("button")!.disabled).toBe(false)
  })
  it("rejects empty and invalid emails without transport", () => {
    const transport = vi.fn(); vi.stubGlobal("fetch", transport)
    const { host, submit } = mount()
    submit(" "); expect(host.textContent).toContain("Missing")
    submit("invalid"); expect(host.textContent).toContain("Invalid")
    expect(transport).not.toHaveBeenCalled()
  })
  it.each([400, 409, 500])("retains status %s handling and input", async status => {
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: false, status })))
    const { host, input, submit } = mount(); submit("reader@example.com"); await settle()
    expect(host.querySelector('[role="alert"]')!.textContent).toBe(status === 500 ? "Failed" : "Already")
    expect(input.value).toBe("reader@example.com")
    expect(host.querySelector("button")!.disabled).toBe(false)
  })
  it("handles rejected inert transport and allows retry", async () => {
    const transport = vi.fn().mockRejectedValueOnce(new Error("inert failure")).mockResolvedValueOnce({ ok: true })
    vi.stubGlobal("fetch", transport)
    const { host, submit } = mount(); submit("reader@example.com"); await settle()
    expect(host.textContent).toContain("Failed")
    submit("reader@example.com"); await settle(); expect(host.textContent).toContain("Subscribed")
  })
})

async function docsRuntime() {
  const context = vm.createContext({ document, window, AbortController: window.AbortController })
  for (const file of ["../components/DocsSideNav.astro", "../layouts/Docs.astro"]) {
    const source = fs.readFileSync(new URL(file, import.meta.url), "utf8")
    const script = source.match(/<script>([\s\S]*?)<\/script>/)![1]
    const code = transpileModule(script, { compilerOptions: { target: 9 } }).outputText
    vm.runInContext(code, context)
  }
}
function docsDom() {
  document.body.innerHTML = '<div class="category" data-expanded="true"><button class="category-header">Category</button></div><button id="docs-menu-toggle"></button><div id="docs-overlay"></div><nav id="docs-sidebar"><a href="/docs">Link</a></nav>'
  return { cat: document.querySelector(".category")!, header: document.querySelector(".category-header")!, toggle: document.getElementById("docs-menu-toggle")!, sidebar: document.getElementById("docs-sidebar")!, overlay: document.getElementById("docs-overlay")!, link: document.querySelector("nav a")! }
}
describe("docs first load and swaps", () => {
  it("binds repeated page-load once and retires detached DOM/global handlers", async () => {
    const first = docsDom(); document.body.style.overflow = "clip"
    await docsRuntime()
    document.dispatchEvent(new Event("astro:page-load")); document.dispatchEvent(new Event("astro:page-load"))
    first.header.dispatchEvent(new Event("click")); expect(first.cat.getAttribute("data-expanded")).toBe("false")
    first.toggle.dispatchEvent(new Event("click")); expect(first.sidebar.classList.contains("open")).toBe(true); expect(document.body.style.overflow).toBe("hidden")
    document.dispatchEvent(new Event("astro:before-swap")); expect(document.body.style.overflow).toBe("clip")
    const next = docsDom(); document.dispatchEvent(new Event("astro:page-load"))
    first.toggle.dispatchEvent(new Event("click")); first.header.dispatchEvent(new Event("click")); expect(first.cat.getAttribute("data-expanded")).toBe("false"); expect(next.sidebar.classList.contains("open")).toBe(false)
    next.header.dispatchEvent(new Event("click")); expect(next.cat.getAttribute("data-expanded")).toBe("false")
    next.toggle.dispatchEvent(new Event("click")); expect(next.toggle.getAttribute("aria-expanded")).toBe("true")
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })); expect(next.sidebar.classList.contains("open")).toBe(false); expect(document.body.style.overflow).toBe("clip")
    next.toggle.dispatchEvent(new Event("click")); next.overlay.dispatchEvent(new Event("click")); expect(next.sidebar.classList.contains("open")).toBe(false)
    Object.defineProperty(window, "innerWidth", { value: 700, configurable: true }); next.toggle.dispatchEvent(new Event("click")); next.link.dispatchEvent(new Event("click")); expect(next.sidebar.classList.contains("open")).toBe(false)
    Object.defineProperty(window, "innerWidth", { value: 1200, configurable: true }); next.toggle.dispatchEvent(new Event("click")); next.link.dispatchEvent(new Event("click")); expect(next.sidebar.classList.contains("open")).toBe(true)
    document.dispatchEvent(new Event("astro:before-swap")); document.body.innerHTML = ""; document.body.style.overflow = "scroll"; document.dispatchEvent(new Event("astro:page-load")); next.toggle.dispatchEvent(new Event("click")); document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })); expect(document.body.style.overflow).toBe("scroll"); document.dispatchEvent(new Event("astro:before-swap")); expect(document.body.style.overflow).toBe("scroll")
  })
})
