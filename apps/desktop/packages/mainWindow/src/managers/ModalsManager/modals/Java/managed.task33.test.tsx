import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import AddManagedJava from "./AddManagedJava"
import { button, deferred, settle, toast } from "@/task33.fixture"
const c = vi.hoisted(() => ({
  mutate: vi.fn(),
  arch: "x64" as any,
  platform: "linux",
  versions: undefined as any,
  vendors: undefined as any
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (f: any) => ({
      get data() {
        return f().queryKey[0] === "java.getManagedVendors"
          ? c.vendors()
          : c.versions()
      },
      isLoading: false
    }),
    createMutation: () => ({ mutateAsync: c.mutate })
  }
}))
let dispose: undefined | (() => void), setVersions: any
beforeEach(() => {
  vi.clearAllMocks()
  c.mutate.mockReset().mockResolvedValue(undefined)
  c.arch = "x64"
  c.platform = "linux"
  const [versions, sv] = createSignal<any>({
    linux: { x64: [{ id: "java17", name: "Java17" }] }
  })
  c.versions = versions
  setVersions = sv
  const [vendors] = createSignal<any>(["azul"])
  c.vendors = vendors
  window.getCurrentOS = vi.fn(
    async () => ({ arch: c.arch, platform: c.platform }) as any
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
async function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <AddManagedJava title="Synthetic" data={{}} />, root)
  await settle()
  return root
}
it("0168 awaited success clears loading and retains one success feedback", async () => {
  const p = deferred()
  c.mutate.mockReturnValue(p.promise)
  const root = await mount()
  const b = button(root, "java:_trn_install")
  b.click()
  await settle()
  expect(b.disabled).toBe(true)
  p.resolve(undefined)
  await settle()
  expect(toast.success).toHaveBeenCalledTimes(1)
  expect(b.disabled).toBe(false)
  b.click()
  await settle()
  expect(c.mutate).toHaveBeenCalledTimes(2)
})
it("0168 rejection releases action without success", async () => {
  const p = deferred()
  c.mutate.mockReturnValue(p.promise)
  const root = await mount()
  const b = button(root, "java:_trn_install")
  b.click()
  p.reject(new Error("synthetic failed setup"))
  await settle()
  expect(b.disabled).toBe(false)
  expect(toast.success).not.toHaveBeenCalled()
})
it("0168 native pending repeat stays inert and direct pending handler is guarded", async () => {
  const p = deferred()
  c.mutate.mockReturnValue(p.promise)
  const root = await mount()
  const b = button(root, "java:_trn_install")
  b.click()
  b.click()
  expect(c.mutate).toHaveBeenCalledTimes(1)
  void (b as any).invoke()
  expect(c.mutate).toHaveBeenCalledTimes(1)
  p.resolve(undefined)
  await settle()
})
it.each(["arch", "platform", "id"])(
  "0168 missing %s cannot start",
  async (missing) => {
    if (missing === "arch") c.arch = undefined
    if (missing === "platform") c.platform = "unsupported"
    if (missing === "id") setVersions({ linux: { x64: [] } })
    const root = await mount()
    const b = button(root, "java:_trn_install")
    b.click()
    await settle()
    expect(c.mutate).not.toHaveBeenCalled()
    expect(b.disabled).toBe(false)
  }
)
