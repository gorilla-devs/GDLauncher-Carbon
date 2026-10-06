import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal, createEffect, onCleanup } from "solid-js"
import { render } from "solid-js/web"
import ModDetails from "./ModDetails"
import { deferred, settle } from "@/task33.fixture"
const c = vi.hoisted(() => ({ read: vi.fn(), keys: [] as any[], disposed: 0 }))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({ useTransContext: () => [(k: string) => k] }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: vi.fn() })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/components/CopyIcon", () => ({ default: () => null }))
vi.mock("@/utils/instances", () => ({ getModImageUrl: () => "" }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (f: any) => {
      const [state, set] = createSignal<any>({
        data: undefined,
        isSuccess: false,
        isPending: true,
        isFetching: true,
        isError: false,
        isPlaceholderData: false
      })
      let generation = 0,
        lastKey = ""
      const request = (o: any) => {
        lastKey = JSON.stringify([o.queryKey, o.enabled])
        const g = ++generation
        c.keys.push(o)
        set({
          data: undefined,
          isPending: true,
          isFetching: true,
          isSuccess: false,
          isError: false,
          isPlaceholderData: false
        })
        if (o.enabled === false) return
        const result = c.read(o.queryKey)
        if (result && typeof result.then !== "function") {
          set({
            data: result,
            isSuccess: true,
            isPending: false,
            isFetching: false,
            isError: false,
            isPlaceholderData: false
          })
          return
        }
        Promise.resolve(result).then(
          (data) => {
            if (g === generation)
              set({
                data,
                isSuccess: true,
                isPending: false,
                isFetching: false,
                isError: false,
                isPlaceholderData: false
              })
          },
          () => {
            if (g === generation)
              set({
                data: undefined,
                isSuccess: false,
                isPending: false,
                isFetching: false,
                isError: true,
                isPlaceholderData: false
              })
          }
        )
      }
      request(f())
      createEffect(() => {
        const o = f()
        if (JSON.stringify([o.queryKey, o.enabled]) !== lastKey) request(o)
      })
      onCleanup(() => {
        ++generation
        ++c.disposed
      })
      return new Proxy({}, { get: (_, k) => state()[k] })
    }
  }
}))
let dispose: undefined | (() => void), setMod: any
beforeEach(() => {
  vi.clearAllMocks()
  c.read.mockReset()
  c.keys = []
  c.disposed = 0
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount(provider = "cf", description = "", project: any = 4) {
  const [mod, set] = createSignal<any>({
    id: "typed-row",
    filename: "owned.jar",
    enabled: true,
    metadata: { name: "Synthetic", description },
    ...(provider === "cf"
      ? { curseforge: { project_id: project, urlslug: "synthetic" } }
      : { modrinth: { project_id: project } })
  })
  setMod = set
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <ModDetails title="Synthetic" data={{ instanceId: 7, mod: mod() }} />,
    root
  )
  return root
}
it.each(["cf", "mr"])(
  "0170 deferred %s current response reaches plain description view",
  async (provider) => {
    const p = deferred()
    c.read.mockReturnValue(p.promise)
    const root = mount(provider, "", provider === "cf" ? 4 : "projectA")
    expect(root.textContent).not.toContain("Current description")
    p.resolve(
      provider === "cf"
        ? { data: { summary: "Current description" } }
        : { description: "Current description" }
    )
    await settle()
    expect(root.textContent).toContain("Current description")
  }
)
it("0170 warm summary remains displayed", async () => {
  c.read.mockReturnValue({ data: { summary: "Warm summary" } })
  const root = mount()
  await settle()
  expect(root.textContent).toContain("Warm summary")
})
it("0170 nonempty stored description remains preferred", async () => {
  c.read.mockReturnValue({ data: { summary: "Provider description" } })
  const root = mount("cf", "Stored description")
  await settle()
  expect(root.textContent).toContain("Stored description")
  expect(root.textContent).not.toContain("Provider description")
})
it("0170 provider/project replacement rejects delayed old response and loads current provider", async () => {
  const a = deferred(),
    b = deferred()
  c.read.mockImplementation((key: any) =>
    key[0].includes("curseforge") ? a.promise : b.promise
  )
  const root = mount()
  await settle()
  setMod({
    id: "new-row",
    filename: "other.jar",
    enabled: true,
    metadata: { name: "New" },
    modrinth: { project_id: "B" }
  })
  await settle()
  a.resolve({ data: { summary: "Stale A" } })
  await settle()
  expect(root.textContent).not.toContain("Stale A")
  b.resolve({ description: "Current B" })
  await settle()
  expect(root.textContent).toContain("Current B")
  expect(
    c.keys.some(
      (o) =>
        o.queryKey[0] === "modplatforms.modrinth.getProject" &&
        o.queryKey[1] === "B" &&
        o.enabled !== false
    )
  ).toBe(true)
})
it("0170 current empty/error stays safe and disposal retires query ownership", async () => {
  const p = deferred()
  c.read.mockReturnValue(p.promise)
  const root = mount()
  await settle()
  p.reject(new Error("synthetic provider denied"))
  await settle()
  expect(root.textContent).not.toContain("undefined")
  dispose?.()
  dispose = undefined
  expect(c.disposed).toBeGreaterThan(0)
})
