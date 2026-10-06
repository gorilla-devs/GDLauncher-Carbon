import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { ThemeProvider, useTheme } from "./theme-context"
import ThemeSelector from "../components/ThemeSelector"
import {
  mainTheme,
  pixelato,
  win95,
  inferno,
  aether,
  frost,
  type Theme
} from "../../../src/themes"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  localStorage.clear()
  document.documentElement.removeAttribute("style")
  document.getElementById("additional-styles")?.remove()
  vi.restoreAllMocks()
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
  await new Promise((r) => setTimeout(r, 30))
}
function fixture(saved?: string, selector = false) {
  if (saved !== undefined) localStorage.setItem("ui-theme", saved)
  let context!: ReturnType<typeof useTheme>
  const Child = () => {
    context = useTheme()
    return selector ? <ThemeSelector /> : <div />
  }
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <ThemeProvider>
        <Child />
      </ThemeProvider>
    ),
    root
  )
  return { context, root }
}
function applied(theme: Theme) {
  for (const [key, value] of Object.entries(theme)) {
    if (key === "additional-styles")
      expect(document.getElementById(key)?.innerHTML).toBe(value)
    else
      expect(document.documentElement.style.getPropertyValue(`--${key}`)).toBe(
        value
      )
  }
}
it.each([
  ["inferno", inferno],
  ["aether", aether],
  ["frost", frost]
] as const)(
  "GPT0182 exported offered %s applies through the provider without throwing",
  (name, theme) => {
    const f = fixture()
    expect(() => f.context.setTheme(name as any)).not.toThrow()
    expect(f.context.currentTheme()).toBe(name)
    expect(f.context.themeObject()).toBe(theme)
    expect(localStorage.getItem("ui-theme")).toBe(name)
    applied(theme)
  }
)
it.each([
  ["main", mainTheme],
  ["pixelato", pixelato],
  ["win95", win95],
  ["inferno", inferno],
  ["aether", aether],
  ["frost", frost]
] as const)("saved registered %s applies its actual export", (name, theme) => {
  const f = fixture(name)
  expect(f.context.currentTheme()).toBe(name)
  applied(theme)
})
it.each(["toString", "constructor", "__proto__", "unknown"])(
  "GPT0264 saved %s falls back to main and applies CSS",
  (name) => {
    const f = fixture(name)
    expect(f.context.currentTheme()).toBe("main")
    applied(mainTheme)
  }
)
it.each(["toString", "constructor", "__proto__", "unknown", undefined])(
  "invalid setter %s preserves the valid selection and stored value",
  (name) => {
    const f = fixture("win95")
    expect(() => f.context.setTheme(name as any)).not.toThrow()
    expect(f.context.currentTheme()).toBe("win95")
    expect(localStorage.getItem("ui-theme")).toBe("win95")
    applied(win95)
  }
)
it.each([
  ["Main Theme", "main", mainTheme],
  ["Pixelato", "pixelato", pixelato],
  ["Windows 95", "win95", win95],
  ["Inferno", "inferno", inferno],
  ["Aether", "aether", aether],
  ["Frost", "frost", frost]
] as const)(
  "actual selector offered %s publishes and persists its registered export",
  async (label, name, theme) => {
    const computed = window.getComputedStyle.bind(window)
    vi.spyOn(window, "getComputedStyle").mockImplementation(
      (element, pseudo) =>
        new Proxy(computed(element, pseudo), {
          get: (target, key) =>
            key === "animationName"
              ? "none"
              : key === "animationDuration"
                ? "0s"
                : Reflect.get(target, key)
        })
    )
    const f = fixture(name === "main" ? "win95" : "main", true)
    const trigger = f.root.querySelector("button")!
    trigger.focus()
    trigger.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })
    )
    await flush()
    const options = [...document.querySelectorAll<HTMLElement>("[role=option]")]
    const optionLabel = (o: HTMLElement) =>
      o.querySelector(".flex-1")?.textContent
    expect(options.map(optionLabel)).toEqual([
      "Main Theme",
      "Pixelato",
      "Windows 95",
      "Inferno",
      "Aether",
      "Frost"
    ])
    const option = options.find((o) => optionLabel(o) === label)!
    const errors: unknown[] = []
    const failed = (event: ErrorEvent) => {
      errors.push(event.error)
      event.preventDefault()
    }
    window.addEventListener("error", failed)
    try {
      option.focus()
      option.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
      )
      await flush()
    } finally {
      window.removeEventListener("error", failed)
    }
    expect(errors).toEqual([])
    expect(f.context.currentTheme()).toBe(name)
    expect(localStorage.getItem("ui-theme")).toBe(name)
    applied(theme)
  }
)
