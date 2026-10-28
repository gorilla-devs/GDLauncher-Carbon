import { afterEach, describe, expect, it, vi } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import vm from "node:vm"
import ts from "typescript"
import { marked } from "marked"
import data, { getAddon, listAddons, type AddonEntry } from "./addonsData"
import { getComparison, COMPETITOR_NAMES, type ComparisonSlug } from "./vsData"

// Resolved from this file, not the working directory, so the test runs from
// the repository root and from apps/website alike.
const src = path.join(path.dirname(fileURLToPath(import.meta.url)), "..")
const source = (rel: string) => fs.readFileSync(path.join(src, rel), "utf8")
function declarations(rel: string, names: string[]) {
  const front = source(rel).split("---")[1]
  const ast = ts.createSourceFile(rel, front, ts.ScriptTarget.Latest, true)
  return ast.statements.filter(n =>
    ts.isFunctionDeclaration(n) ? names.includes(n.name!.text) :
      ts.isVariableStatement(n) && n.declarationList.declarations.some(d => names.includes(d.name.getText(ast)))
  ).map(n => n.getText(ast)).join("\n")
}
function compile(code: string) {
  return ts.transpileModule(code, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.React } }).outputText
}
const interpolation = vm.runInNewContext(compile(declarations("components/AddonInstallLayout.astro", ["escapeHtml", "interp", "interpHtml"])) + "\n({interp,interpHtml})")
const classFor = vm.runInNewContext(compile(declarations("components/ComparisonPageLayout.astro", ["classFor"])) + "\nclassFor")
let retireArticle: (() => void) | undefined
const listeners: Array<[string, EventListenerOrEventListenerObject]> = []
afterEach(() => {
  retireArticle?.(); retireArticle = undefined
  document.dispatchEvent(new Event("astro:before-swap"))
  for (const [name, listener] of listeners) document.removeEventListener(name, listener)
  listeners.length = 0; document.body.innerHTML = ""; vi.restoreAllMocks()
})

describe("Task42 literal addon interpolation", () => {
  it.each([
    ["Test $& Mod", "Install Test $& Mod"],
    ["Test $$ Mod", "Install Test $$ Mod"],
    ["Test $` Mod", "Install Test $` Mod"],
    ["Test $' Mod", "Install Test $' Mod"],
  ])("inserts %s literally", (name, expected) => expect(interpolation.interp("Install {name}", { name })).toBe(expected))
  it("escapes inserted HTML values while retaining template markup and dollars", () => {
    expect(interpolation.interpHtml("<strong>{name}</strong>", { name: 'Test $& <b>"Mod"</b>' })).toBe('<strong>Test $&amp; &lt;b&gt;&quot;Mod&quot;&lt;/b&gt;</strong>')
  })
  it("does not reinterpret inserted placeholders", () => {
    expect(interpolation.interp("{name} {type}", { name: "{type}", type: "Mod" })).toBe("{type} Mod")
    expect(interpolation.interpHtml("{name} {type}", { name: "{type}", type: "<Mod>" })).toBe("{type} &lt;Mod&gt;")
  })
  it("preserves ordinary repeated, number and unknown placeholders", () => {
    expect(interpolation.interp("{name}/{name} {count} {missing}", { name: "Normal", count: 7 })).toBe("Normal/Normal 7 {missing}")
    expect(interpolation.interp("{name}", { name: "A & <B>" })).toBe("A & <B>")
    expect(interpolation.interpHtml("{name}", { name: "A & <B>" })).toBe("A &amp; &lt;B&gt;")
  })
})

describe("Task42 stored own addon slugs", () => {
  it.each(["constructor", "toString", "__proto__"])("rejects inherited %s", slug => {
    expect(Object.hasOwn(data.modrinth.mods!, slug)).toBe(false)
    expect(getAddon("modrinth", "mods", slug)).toBeUndefined()
  })
  it("retains known, absent, unavailable-combination and sparse list controls", () => {
    expect(getAddon("modrinth", "mods", "fabric-api")).toBe(data.modrinth.mods!["fabric-api"])
    expect(getAddon("modrinth", "mods", "fixture-missing-42")).toBeUndefined()
    expect(getAddon("modrinth", "worlds", "fabric-api")).toBeUndefined()
    expect(listAddons("modrinth", "worlds")).toEqual([])
    expect(listAddons("modrinth", "mods")).toEqual(Object.values(data.modrinth.mods!))
  })
  it.each(["constructor", "toString", "__proto__"])("retains explicitly stored own %s", slug => {
    const map = data.modrinth.mods!
    const record: AddonEntry = { slug, name: "Owned Fixture", imageUrl: null, websiteUrl: "https://example.invalid/" }
    Object.defineProperty(map, slug, { value: record, configurable: true, enumerable: true })
    try { expect(getAddon("modrinth", "mods", slug)).toBe(record); expect(listAddons("modrinth", "mods")).toContain(record) }
    finally { delete map[slug] }
  })
})

async function article(html: string) {
  document.body.innerHTML = html
  let mounted: Promise<unknown> | undefined
  const exports: Record<string, any> = {}
  const context = vm.createContext({ document, history, React: { Fragment: "fragment", createElement: () => null }, exports, require: () => ({
    onMount(fn: () => unknown) { mounted = Promise.resolve(fn()) },
    onCleanup(fn: () => void) { retireArticle = fn },
  }) })
  vm.runInContext(compile(source("components/ArticleInteractiveMarkdown/index.tsx")), context)
  exports.default()
  await mounted
  return async () => { exports.default(); await mounted }
}
describe("Task42 optional article TOC", () => {
  it.each(["best-modpacks-2026", "best-shaders-2026"])("safely initializes the current post without a TOC: %s", async slug => {
    const body = source(`content/blog/${slug}.md`).replace(/^---\n[\s\S]*?\n---\n/, "")
    await article(`<article id="articleContainer">${marked(body)}</article>`)
    expect(document.getElementById("tableList")).toBeNull()
    expect(document.querySelectorAll("h2").length).toBeGreaterThan(0)
  })
  it("initializes supported headings without a TOC and retains heading click effects", async () => {
    await article('<article id="articleContainer"><h1 id="title">Title</h1><h2 id="child">Child</h2></article>')
    const h1 = document.querySelector("h1")!, h2 = document.querySelector("h2")!
    const push = vi.spyOn(history, "pushState")
    h1.querySelector("svg")!.dispatchEvent(new Event("click", { bubbles: true }))
    expect(push).toHaveBeenLastCalledWith({}, "", "#title")
    h2.click(); expect(push).toHaveBeenLastCalledWith({}, "", "#child")
  })
  it("preserves legacy TOC, authored nodes, navigation and single generated ownership on remount", async () => {
    const remount = await article('<summary id="tableListSummary">Contents</summary><ul id="tableList"><li id="authored">Authored</li></ul><article id="articleContainer"><h1 id="title">Title</h1><h2 id="child"><em>Child</em></h2></article>')
    const h1 = document.querySelector("h1")!, h2 = document.querySelector("h2")!
    const scroll1 = vi.fn(), scroll2 = vi.fn(); h1.scrollIntoView = scroll1; h2.scrollIntoView = scroll2
    const push = vi.spyOn(history, "pushState")
    expect(document.getElementById("tableList")!.style.marginLeft).toBe("10px")
    expect(document.getElementById("tableListSummary")!.style.cursor).toBe("pointer")
    document.querySelector<HTMLAnchorElement>("#tableList > li:nth-child(2) > a")!.click()
    expect(scroll1).toHaveBeenCalledWith({ behavior: "smooth" })
    document.querySelector<HTMLLIElement>("#tableList ul li")!.click(); expect(scroll2).toHaveBeenCalledWith({ behavior: "smooth" })
    const obsoleteCleanup = retireArticle
    await remount(); await remount(); obsoleteCleanup?.()
    expect(h1.querySelectorAll("svg")).toHaveLength(1); expect(h2.querySelectorAll("svg")).toHaveLength(1)
    expect(document.querySelectorAll("#tableList > li")).toHaveLength(2)
    expect(document.getElementById("authored")!.textContent).toBe("Authored"); expect(h2.querySelector("em")!.textContent).toBe("Child")
    push.mockClear(); h2.click(); expect(push).toHaveBeenCalledTimes(1)
    retireArticle?.()
    expect(h1.querySelectorAll("svg")).toHaveLength(0); expect(h2.querySelectorAll("svg")).toHaveLength(0)
    expect(document.querySelectorAll("#tableList > li")).toHaveLength(1)
    expect(document.getElementById("authored")!.textContent).toBe("Authored")
  })
  it("safely handles absent container and headings", async () => { await article(""); await article('<article id="articleContainer"><p>Plain</p></article>') })
})

function showcase(count: number, observerAvailable = true) {
  document.body.innerHTML = Array.from({ length: count }, (_, i) => `<video data-lazy-src="https://media.example.invalid/${i}.mp4"></video>`).join("")
  const videos = [...document.querySelectorAll("video")]
  for (const video of videos) {
    Object.defineProperty(video, "paused", { value: false, configurable: true })
    video.load = vi.fn(); video.play = vi.fn(async () => {}); video.pause = vi.fn()
  }
  const observers: TrackingObserver[] = []
  class TrackingObserver {
    targets = new Set<Element>(); disconnect = vi.fn(() => this.targets.clear())
    constructor(public callback: (entries: any[]) => void) { observers.push(this) }
    observe(target: Element) { this.targets.add(target) }
  }
  const add = document.addEventListener.bind(document)
  vi.spyOn(document, "addEventListener").mockImplementation((type, listener, options) => { listeners.push([type, listener]); add(type, listener, options) })
  const code = source("components/landing/FeaturesShowcase.astro").match(/<script>([\s\S]*?)<\/script>/)![1]
  const context = vm.createContext({ document, window: observerAvailable ? { IntersectionObserver: TrackingObserver } : {}, IntersectionObserver: TrackingObserver })
  vm.runInContext(compile(code), context)
  return { videos, observers, setup: () => vm.runInContext("setup()", context) }
}
describe("Task42 showcase observer ownership", () => {
  it("retires every observer after initial and repeated page-load followed by swap", () => {
    const { observers } = showcase(4)
    document.dispatchEvent(new Event("astro:page-load")); document.dispatchEvent(new Event("astro:page-load"))
    expect(observers.filter(o => o.targets.size > 0)).toHaveLength(1)
    document.dispatchEvent(new Event("astro:before-swap")); expect(observers.every(o => o.targets.size === 0)).toBe(true)
  })
  it("retires the previous observer before a zero-video replacing setup", () => {
    const { observers, setup } = showcase(1); document.body.innerHTML = ""; setup()
    expect(observers.every(o => o.targets.size === 0)).toBe(true)
  })
  it("has no observer for an empty page", () => { const { observers } = showcase(0); document.dispatchEvent(new Event("astro:page-load")); document.dispatchEvent(new Event("astro:before-swap")); expect(observers).toHaveLength(0) })
  it("loads lazy media once, plays intersections, pauses exits and handles rejection", async () => {
    const { videos: [video], observers: [observer] } = showcase(1)
    video.play = vi.fn(() => Promise.reject(new Error("inert autoplay denial")))
    observer.callback([{ target: video, isIntersecting: true }]); observer.callback([{ target: video, isIntersecting: true }])
    await Promise.resolve(); expect(video.src).toBe("https://media.example.invalid/0.mp4"); expect(video.load).toHaveBeenCalledTimes(1); expect(video.play).toHaveBeenCalledTimes(2)
    observer.callback([{ target: video, isIntersecting: false }]); expect(video.pause).toHaveBeenCalledTimes(1)
  })
  it("preserves fallback play for one intended setup without native media", () => { const { videos: [video], observers } = showcase(1, false); expect(video.load).toHaveBeenCalledTimes(1); expect(video.play).toHaveBeenCalledTimes(1); expect(observers).toHaveLength(0) })
  it("observes only fresh nodes after replacement", () => {
    const { observers, setup } = showcase(1); const old = document.querySelector("video")!
    document.body.innerHTML = '<video data-lazy-src="https://media.example.invalid/new.mp4"></video>'; setup()
    expect(observers[0].targets.size).toBe(0); expect(observers[1].targets.has(old)).toBe(false); expect(observers[1].targets.has(document.querySelector("video")!)).toBe(true)
  })
})

describe("Task42 Italian comparison styling", () => {
  it.each(["Sì", "Sì (integrata)"])("styles %s positively", value => expect(classFor(value).color).toBe("text-green-400"))
  it("keeps locale affirmative, negative, partial and nonaffirmative boundary controls", () => {
    for (const value of ["Yes", "Yes (built in)", "Sí", "Oui", "Ja", "Sim", "예", "あり"]) expect(classFor(value).color).toBe("text-green-400")
    for (const value of ["No", "Nein", "Non", "Não", "なし", "아니요"]) expect(classFor(value).color).toBe("text-darkSlate-400")
    for (const value of ["Partial", "Parziale (con workaround)", "sìness", "sìno"]) expect(classFor(value).color).toBe("text-yellow-400")
  })
  it("classifies actual Italian affirmative cells and preserves Prism partial qualifier", () => {
    let affirmative = 0, translated = 0, fallback = 0
    for (const slug of Object.keys(COMPETITOR_NAMES) as ComparisonSlug[]) {
      const rows = getComparison("it", slug).rows
      if (rows.some(row => [row.gdl, row.competitor].some(v => /^sì(?:$|\s)/i.test(v)))) translated++; else fallback++
      for (const row of rows) for (const value of [row.gdl, row.competitor]) if (/^sì(?:$|\s)/i.test(value)) { affirmative++; expect(classFor(value).color).toBe("text-green-400") }
    }
    expect(translated).toBe(5); expect(fallback).toBe(2); expect(affirmative).toBeGreaterThan(0)
    const prism = getComparison("it", "prismlauncher").rows.find(row => row.competitor === "Parziale (con workaround)")!
    expect(prism.note).toBeTruthy(); expect(classFor(prism.competitor).color).toBe("text-yellow-400")
    console.log(JSON.stringify({ italianAffirmativeCells: affirmative, translatedComparisons: translated, englishFallbackComparisons: fallback }))
  })
})
