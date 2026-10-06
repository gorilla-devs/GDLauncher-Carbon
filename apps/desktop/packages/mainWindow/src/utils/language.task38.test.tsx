import { afterEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import changeLanguage from "./language"
const c = vi.hoisted(() => ({
  writes: [] as string[],
  registrations: [] as string[],
  switches: [] as string[],
  events: [] as string[]
}))
vi.mock("@gd/i18n", async (importOriginal) => ({
  ...(await importOriginal<any>()),
  useTransContext: () => [
    () => "",
    {
      addResources: (code: string, ns: string) => {
        c.registrations.push(code)
        c.events.push(`resources:${ns}`)
      },
      changeLanguage: (code: string) => {
        c.switches.push(code)
        c.events.push("switch")
      }
    }
  ]
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    createMutation: () => ({
      mutateAsync: async (patch: any) => {
        c.writes.push(patch.language.Set)
        c.events.push("save")
      }
    })
  }
}))
afterEach(() => {
  c.writes = []
  c.registrations = []
  c.switches = []
  c.events = []
})
it.each([
  ["english", "en"],
  ["italian", "it"],
  ["japanese", "ja"],
  ["russian", "ru"],
  ["spanish", "es"],
  ["simplified_chinese", "zh-CN"],
  ["french", "fr"],
  ["toString", "en"],
  ["__proto__", "en"],
  ["not-supported", "en"]
])(
  "GPT0137 actual switch %s persists supported word ID and publishes mapped %s after resources/save",
  async (id, code) => {
    await createRoot((dispose) => {
      const promise = changeLanguage(id)
      dispose()
      return promise
    })
    expect(c.writes).toEqual([code === "en" ? "english" : id])
    expect(c.registrations.length).toBeGreaterThan(0)
    expect(new Set(c.registrations)).toEqual(new Set([code]))
    expect(c.switches).toEqual([code])
    const saveIndex = c.events.indexOf("save")
    expect(saveIndex).toBe(c.events.length - 2)
    expect(c.events.at(-1)).toBe("switch")
  }
)
