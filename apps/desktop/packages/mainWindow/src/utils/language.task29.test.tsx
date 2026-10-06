import { afterEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { deferred, settle } from "../task29.fixture"
import changeLanguage from "./language"

const control = vi.hoisted(() => ({
  loaders: new Map<string, any>(),
  displayed: "it",
  persisted: "italian",
  writes: [] as string[],
  saves: new Map<string, any>(),
  add: vi.fn()
}))
vi.mock("@gd/i18n", async (importOriginal) => ({
  ...(await importOriginal<any>()),
  loadLanguageFiles: (language: string) =>
    control.loaders.get(language).promise,
  useTransContext: () => [
    () => "",
    {
      addResources: control.add,
      changeLanguage: (language: string) => {
        control.displayed = language
      }
    }
  ]
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    createMutation: () => {
      const save = async (patch: any) => {
        const language = patch.language.Set
        control.writes.push(language)
        // Persistence happens when this synthetic operation commits, before its
        // response. A gate permits an attempted reverse commit schedule.
        await control.saves.get(language)?.promise
        control.persisted = language
      }
      return {
        mutate: (patch: any) => {
          void save(patch).catch(() => {})
        },
        mutateAsync: save
      }
    }
  }
}))
afterEach(async () => {
  for (const value of control.saves.values()) value.resolve(undefined)
  await settle()
  control.loaders.clear()
  control.saves.clear()
  control.writes = []
  control.displayed = "it"
  control.persisted = "italian"
  vi.clearAllMocks()
})
const request = (language: string) =>
  createRoot((dispose) => {
    const result = changeLanguage(language)
    dispose()
    return result
  })
it("0193 latest selection wins reversed resource completion and partial fallback", async () => {
  control.loaders.set("russian", deferred())
  control.loaders.set("english", deferred())
  const old = request("russian")
  const latest = request("english")
  control.loaders.get("english").resolve({})
  await latest
  control.loaders.get("russian").resolve({ common: { hello: "synthetic" } })
  await old
  expect(control.displayed).toBe("en")
  expect(control.persisted).toBe("english")
  expect(control.writes).toEqual(["english"])
})
it("0193 shared queue prevents attempted reversed persistence commit order", async () => {
  control.loaders.set("russian", deferred())
  control.loaders.set("english", deferred())
  const oldCommit = deferred()
  const newCommit = deferred()
  control.saves.set("russian", oldCommit)
  control.saves.set("english", newCommit)
  const old = request("russian")
  control.loaders.get("russian").resolve({})
  await settle()
  expect(control.writes).toEqual(["russian"])
  const latest = request("english")
  control.loaders.get("english").resolve({})
  newCommit.resolve(undefined)
  await settle()
  expect(control.writes).toEqual(["russian"])
  expect(control.persisted).toBe("italian")
  oldCommit.resolve(undefined)
  await Promise.all([old, latest])
  expect(control.writes).toEqual(["russian", "english"])
  expect(control.persisted).toBe("english")
  expect(control.displayed).toBe("en")
})
it("0193 failed save cannot poison the latest publication", async () => {
  control.loaders.set("russian", deferred())
  control.loaders.set("english", deferred())
  const failed = deferred()
  control.saves.set("russian", failed)
  const old = request("russian")
  const handled = old.catch(() => {})
  control.loaders.get("russian").resolve({})
  await settle()
  const latest = request("english")
  control.loaders.get("english").resolve({})
  failed.reject(new Error("synthetic save failure"))
  await handled
  await latest
  expect(control.persisted).toBe("english")
  expect(control.displayed).toBe("en")
})
it("0193 in-order selections retain current publication", async () => {
  for (const language of ["russian", "english"]) {
    const loader = deferred()
    control.loaders.set(language, loader)
    const result = request(language)
    loader.resolve({})
    await result
  }
  expect(control.persisted).toBe("english")
  expect(control.displayed).toBe("en")
})
