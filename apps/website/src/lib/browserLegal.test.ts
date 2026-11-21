import { afterEach, describe, expect, it, vi } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import vm from "node:vm"
import { transpileModule } from "typescript"
import { loadLegalDocument } from "./legalDocument.mjs"

const script = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "../components/AddonInstallLayout.astro"), "utf8").match(/<script>([\s\S]*?)<\/script>/)![1]
const runtime = transpileModule(script, {compilerOptions:{target:9}}).outputText
let context: vm.Context
let timers = new Map<number, () => void>()
let next = 0
const copied: string[] = []
const listeners: Array<[string, EventListenerOrEventListenerObject]> = []
function dom(slug="alpha",copy="Copy",done="Copied!") {
  document.body.innerHTML=`<button id="copy-slug-btn" data-slug="#${slug}" data-copy-label="${copy}" data-copied-label="${done}"><svg><path></path></svg><span>${copy}</span></button>`
  return document.querySelector("button")!
}
function mount(writeText: (value:string)=>Promise<unknown> = async value => { copied.push(value) }) {
  const add = document.addEventListener.bind(document)
  vi.spyOn(document, "addEventListener").mockImplementation((type, listener, options) => { listeners.push([type,listener]); add(type,listener,options) })
  context=vm.createContext({document,navigator:{clipboard:{writeText}},AbortController:window.AbortController,setTimeout(fn:()=>void){const id=++next;timers.set(id,fn);return id},clearTimeout(id:number){timers.delete(id)}})
  vm.runInContext(runtime,context)
}
const settle = async () => { await Promise.resolve(); await Promise.resolve() }
function swap(slug="beta",copy="Copier",done="Copié !") {document.dispatchEvent(new Event("astro:before-swap"));const button=dom(slug,copy,done);document.dispatchEvent(new Event("astro:page-load"));return button}
afterEach(()=>{document.dispatchEvent(new Event("astro:before-swap"));document.body.innerHTML="";copied.length=0;timers.clear();next=0;for(const [name, listener] of listeners) document.removeEventListener(name, listener);listeners.length=0;vi.restoreAllMocks()})
describe("Copy lifecycle",()=>{
 it("copies initial alpha once and restores its captured localized label",async()=>{const button=dom("alpha","Kopieren","Kopiert!");mount();button.querySelector("span")!.click();await settle();expect(copied).toEqual(["#alpha"]);expect(button.textContent).toBe("Kopiert!");expect(timers.size).toBe(1);[...timers.values()][0]();expect(button.textContent).toBe("Kopieren")})
 it("copies swapped beta from a nested click and repeated lifecycle once",async()=>{const alpha=dom();mount();alpha.click();await settle();const beta=swap();document.dispatchEvent(new Event("astro:page-load"));document.dispatchEvent(new Event("astro:page-load"));beta.querySelector("path")!.dispatchEvent(new Event("click",{bubbles:true}));await settle();expect(copied).toEqual(["#alpha","#beta"]);expect(beta.textContent).toBe("Copié !");alpha.click();await settle();expect(copied).toEqual(["#alpha","#beta"])})
 it("fresh beta initializes and a page without a button is inert",async()=>{document.body.innerHTML="";mount();document.dispatchEvent(new Event("astro:page-load"));const button=swap("beta","Copy","Copied!");button.click();await settle();expect(copied).toEqual(["#beta"]);swap();document.dispatchEvent(new Event("astro:before-swap"));document.body.innerHTML="";document.dispatchEvent(new Event("astro:page-load"));expect(timers.size).toBe(0)})
 it("clipboard rejection keeps Copy with no success timer",async()=>{const button=dom();mount(async()=>{throw new Error("inert denial")});button.click();await settle();expect(button.textContent).toBe("Copy");expect(timers.size).toBe(0)})
 it("old pending clipboard completion cannot mutate a swapped control",async()=>{let finish!:(v?:unknown)=>void;const alpha=dom();mount(value=>{copied.push(value);return new Promise(r=>{finish=r})});alpha.click();const beta=swap();finish();await settle();expect(copied).toEqual(["#alpha"]);expect(alpha.textContent).toBe("Copy");expect(beta.textContent).toBe("Copier");expect(timers.size).toBe(0)})
 it("a retired feedback timer cannot reset the new page",async()=>{const alpha=dom();mount();alpha.click();await settle();const old=[...timers.values()][0];const beta=swap();beta.click();await settle();old();expect(beta.textContent).toBe("Copié !");expect(timers.size).toBe(1)})
 it("newer rejection keeps its label when an older write resolves",async()=>{let finish!:(v?:unknown)=>void;let writes=0;const button=dom();mount(value=>{copied.push(value);return ++writes===1?new Promise(r=>{finish=r}):Promise.reject(new Error("inert denial"))});button.click();button.dataset.slug="#changed";button.click();await settle();finish();await settle();expect(copied).toEqual(["#alpha","#changed"]);expect(button.textContent).toBe("Copy");expect(timers.size).toBe(0)})
})
describe("legal status before parsing",()=>{
 it("uses the real Markdown parser for successful injected content",async()=>{const html=await loadLegalDocument("https://legal.example.invalid/doc",async()=>new Response("# Fixture Legal\n\n**Valid** terms.",{status:200}));expect(html).toBe("<h1>Fixture Legal</h1>\n<p><strong>Valid</strong> terms.</p>\n")})
 it.each([404,503])("refuses %s before consuming the actual response body",async status=>{const response=new Response("Synthetic unavailable",{status});await expect(loadLegalDocument("https://legal.example.invalid/doc",async()=>response)).rejects.toThrow(`HTTP ${status}`);expect(response.bodyUsed).toBe(false)})
 it("preserves transport rejection",async()=>{const failure=new Error("inert transport");await expect(loadLegalDocument("https://legal.example.invalid/doc",async()=>{throw failure})).rejects.toBe(failure)})
})
