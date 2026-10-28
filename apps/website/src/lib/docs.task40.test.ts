import { afterEach, expect, it } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import vm from "node:vm"
import { transpileModule } from "typescript"
// Resolved from this file, not the working directory, so the test runs from
// the repository root and from apps/website alike.
const here = path.dirname(fileURLToPath(import.meta.url))
const source = fs.readFileSync(path.join(here, "../layouts/Docs.astro"), "utf8")
afterEach(() => { document.body.innerHTML = ""; document.head.innerHTML = "" })
function mount(width: number) {
  Object.defineProperty(window, "innerWidth", { value: width, configurable: true })
  const raw = document.createElement("style"); raw.textContent = source.match(/<style is:global>([\s\S]*?)<\/style>/)![1]; document.head.append(raw)
  const css: string[] = []
  for (const rule of Array.from(raw.sheet!.cssRules)) {
    if (rule.type === 4) {
      const media = rule as CSSMediaRule
      const min = media.conditionText.match(/min-width: (\d+)px/); const max = media.conditionText.match(/max-width: (\d+)px/)
      if ((!min || width >= +min[1]) && (!max || width <= +max[1])) css.push(...Array.from(media.cssRules).map(r => r.cssText))
    } else css.push(rule.cssText)
  }
  raw.remove(); const active = document.createElement("style"); active.textContent = css.join("\n"); document.head.append(active)
  document.body.innerHTML = '<aside id="docs-sidebar" class="docs-sidebar"><a href="/docs">Link</a></aside><div id="docs-overlay" class="docs-overlay"></div><button id="docs-menu-toggle"></button>'
  document.body.style.overflow = "clip"
  vm.runInContext(transpileModule(source.match(/<script>([\s\S]*?)<\/script>/)![1], { compilerOptions: { target: 9 } }).outputText, vm.createContext({ document, window, AbortController: window.AbortController }))
  return { sidebar: document.getElementById("docs-sidebar")!, toggle: document.getElementById("docs-menu-toggle")!, overlay: document.getElementById("docs-overlay")! }
}
it("renders the open sidebar at mobile boundary and hides closed navigation", () => {
  const { sidebar, toggle, overlay } = mount(1023)
  expect(getComputedStyle(sidebar).display).toBe("none")
  toggle.click(); expect(getComputedStyle(sidebar).display).toBe("block")
  expect(toggle.getAttribute("aria-expanded")).toBe("true"); expect(document.body.style.overflow).toBe("hidden")
  overlay.click(); expect(getComputedStyle(sidebar).display).toBe("none"); expect(document.body.style.overflow).toBe("clip")
  toggle.click(); document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }))
  expect(getComputedStyle(sidebar).display).toBe("none"); expect(toggle.getAttribute("aria-expanded")).toBe("false")
  document.dispatchEvent(new Event("astro:before-swap")); document.body.style.overflow = "scroll"; document.body.innerHTML = ""; document.dispatchEvent(new Event("astro:page-load")); document.dispatchEvent(new Event("astro:before-swap")); expect(document.body.style.overflow).toBe("scroll")
})
it("retains desktop visibility at the desktop boundary", () => {
  const { sidebar } = mount(1024); expect(getComputedStyle(sidebar).display).toBe("block")
  document.dispatchEvent(new Event("astro:before-swap"))
})

import { getImportErrorTranslationKey } from "../../../../packages/i18n/helpers"
import { createI18nOptions } from "../../../../packages/i18n/index"
import { createInstance } from "i18next"
it.each(["english", "italian", "japanese", "russian", "spanish", "simplified_chinese", "french"])("delivers the typed unsupported-pack reason in %s", async language => {
  const read = (lang: string) => JSON.parse(fs.readFileSync(path.join(here, `../../../../packages/i18n/locale/${lang}/instances.json`), "utf8"))
  const target = read(language)
  const translator = createInstance()
  await translator.init(createI18nOptions(language, { instances: read("english") }, { instances: target }))
  const key = getImportErrorTranslationKey("InstanceImportGdlpackOptionalUnsupported")
  expect(key).toBe("instances:_trn_import_gdlpack_optional_unsupported")
  expect(translator.t(key)).toBe(target._trn_import_gdlpack_optional_unsupported)
  expect(translator.t(key)).not.toBe(key)
  expect(getImportErrorTranslationKey("InstanceImportGdlpackMalformedManifest")).toBe("instances:_trn_import_error_archive_malformed_manifest")
})
