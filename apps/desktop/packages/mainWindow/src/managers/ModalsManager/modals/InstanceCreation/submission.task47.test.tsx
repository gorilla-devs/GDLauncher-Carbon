import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import { deferred, input, settle, toast } from "@/task31.fixture"
import InstanceCreation from "."
const c = vi.hoisted(() => ({
  store: null as any,
  queries: {} as any,
  mutate: vi.fn(),
  fetch: vi.fn(),
  close: vi.fn(),
  closeOwn: vi.fn(() => true),
  navigate: vi.fn(),
  prevent: undefined as undefined | (() => boolean),
  handlers: new Map<string, () => void>(),
  picker: null as any
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("@/task31.fixture")),
  ...(await import("../../../../../../../../../packages/ui/src/Tabs/index")),
  Button: (p: any) => {
    const { Button } = ui
    c.handlers.set(p["data-testid"], p.onClick)
    return <Button {...p} />
  }
}))
import * as ui from "../../../../../../../../../packages/ui/src/Button/index"
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.store
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close }),
  useModalStackEntry: () => ({
    closeOwn: c.closeOwn,
    registerPreventClose: (f: any) => {
      c.prevent = f
    },
    unregisterPreventClose: () => {
      c.prevent = undefined
    }
  })
}))
vi.mock("@/utils/adhelper", () => ({ default: { width: 0 } }))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@/components/ImagePicker", () => ({
  default: (p: any) => {
    c.picker = p
    return <span data-preview={p.imageUrl()} />
  }
}))
vi.mock("./Import", () => ({ default: () => <span>import fixture</span> }))
vi.mock("./ShareCodeImport", () => ({
  default: () => <span>share fixture</span>
}))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "icon" }))
vi.mock("@/utils/helpers", () => ({
  getModloaderDisplayName: (v: string) => v,
  blobToBase64: async () => "data:image/png;base64,test"
}))
vi.mock("@/utils/rspcClient", () => ({
  apiFetch: (...args: any[]) => c.fetch(...args),
  rspc: {
    createQuery: (f: any) => c.queries[f().queryKey[0]],
    createMutation: (f: any) => ({
      mutateAsync: (v: any) => c.mutate(f().mutationKey[0], v)
    })
  }
}))
let root: HTMLDivElement, dispose: (() => void) | undefined
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
  )
  vi.clearAllMocks()
  c.handlers.clear()
  c.prevent = undefined
  c.closeOwn.mockReturnValue(true)
  c.mutate.mockReset()
  c.mutate.mockImplementation(async (key) =>
    key === "instance.createInstance" ? 7 : undefined
  )
  c.fetch.mockReset()
  c.fetch.mockResolvedValue({ ok: true, blob: async () => new Blob() })
  c.store = createStore({
    minecraftVersions: { data: [{ id: "1.21", type: "release" }] }
  })[0]
  c.queries = { "instance.getDefaultGroup": { data: 3 } }
  for (const key of [
    "mc.getForgeVersions",
    "mc.getNeoforgeVersions",
    "mc.getFabricVersions",
    "mc.getQuiltVersions"
  ])
    c.queries[key] = { data: { gameVersions: [] }, refetch: vi.fn() }
  root = document.createElement("div")
  document.body.append(root)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
  vi.unstubAllGlobals()
})
const submit = () =>
  root.querySelector<HTMLButtonElement>(
    '[data-testid="instance-creation-submit"]'
  )!
function mount() {
  dispose = render(
    () => <InstanceCreation title="Synthetic" data={{ groupId: 9 }} />,
    root
  )
  input(root, 0, "Owned draft")
}
const count = (key: string) =>
  c.mutate.mock.calls.filter((x) => x[0] === `instance.${key}`)
const invoke = (id = "instance-creation-submit") => c.handlers.get(id)!()
const retry = () =>
  root.querySelector<HTMLButtonElement>(
    '[data-testid="instance-creation-retry"]'
  )!
test.each(["icon", "create", "prepare"])(
  "pending %s admits one coherent chain and protects actual Button, tabs and modal close",
  async (stage) => {
    const gate = deferred()
    mount()
    await c.picker.onSelect("/synthetic/icon.png")
    if (stage === "icon") c.fetch.mockImplementation(() => gate.promise)
    else
      c.mutate.mockImplementation((key) =>
        key ===
        `instance.${stage === "create" ? "createInstance" : "prepareInstance"}`
          ? gate.promise
          : Promise.resolve(7)
      )
    invoke()
    invoke()
    await settle()
    expect(
      c.fetch.mock.calls.filter((x) => x[0].includes("loadIcon"))
    ).toHaveLength(1)
    expect(count("createInstance")).toHaveLength(stage === "icon" ? 0 : 1)
    expect(count("prepareInstance")).toHaveLength(stage === "prepare" ? 1 : 0)
    expect(submit().disabled).toBe(true)
    expect(submit().querySelector("svg, .animate-spin")).not.toBeNull()
    expect(c.prevent?.()).toBe(true)
    expect(
      [...root.querySelectorAll<HTMLButtonElement>('[role="tab"]')].every(
        (b) => b.disabled
      )
    ).toBe(true)
    root.querySelector<HTMLElement>('[data-testid="modal-close"]')!.click()
    expect(c.close).not.toHaveBeenCalled()
    submit().click()
    submit().dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
    )
    // A direct event bypass cannot change the accepted name/group after upload.
    input(root, 0, "Later draft")
    gate.resolve(stage === "icon" ? { ok: true } : 7)
    await settle()
    expect(count("createInstance")[0][1]).toMatchObject({
      name: "Owned draft",
      group: 9
    })
    expect(count("prepareInstance")).toEqual([["instance.prepareInstance", 7]])
    expect(c.closeOwn).toHaveBeenCalledTimes(1)
    expect(c.close).not.toHaveBeenCalled()
    expect(c.navigate).toHaveBeenCalledTimes(1)
  }
)
test("blank title repeated submit admits nothing", async () => {
  mount()
  input(root, 0, "")
  invoke()
  invoke()
  await settle()
  expect(c.mutate).not.toHaveBeenCalled()
})
test("icon rejection preserves editable title/icon/version and allows another explicit submit", async () => {
  mount()
  await c.picker.onSelect("/synthetic/icon.png")
  c.fetch.mockRejectedValueOnce(Error("upload failed"))
  invoke()
  await settle()
  expect(root.querySelector<HTMLInputElement>("input")!.value).toBe(
    "Owned draft"
  )
  expect(c.picker.imageUrl()).toContain("base64,test")
  expect(count("createInstance")).toHaveLength(0)
  expect(c.close).not.toHaveBeenCalled()
  expect(submit().disabled).toBe(false)
  invoke()
  await settle()
  expect(count("createInstance")).toHaveLength(1)
})
test("lost create acknowledgement retains draft and requires Library reconciliation or explicit new-copy intent", async () => {
  mount()
  c.mutate.mockRejectedValueOnce(Error("connection lost"))
  invoke()
  await settle()
  expect(root.querySelector<HTMLInputElement>("input")!.value).toBe(
    "Owned draft"
  )
  expect(c.close).not.toHaveBeenCalled()
  expect(submit().disabled).toBe(true)
  invoke()
  await settle()
  expect(count("createInstance")).toHaveLength(1)
  expect(
    root.querySelector('[data-testid="instance-creation-library"]')
  ).not.toBeNull()
  root
    .querySelector<HTMLButtonElement>(
      '[data-testid="instance-creation-fresh-copy"]'
    )!
    .click()
  invoke()
  await settle()
  expect(count("createInstance")).toHaveLength(2)
})
test("returned ID survives prepare rejection and tab changes; repeated retries prepare that ID without icon/create", async () => {
  mount()
  await c.picker.onSelect("/synthetic/icon.png")
  c.mutate.mockImplementation(async (key) => {
    if (key === "instance.createInstance") return 7
    throw Error("runtime admission rejected")
  })
  invoke()
  await settle()
  expect(root.querySelector<HTMLInputElement>("input")!.value).toBe(
    "Owned draft"
  )
  expect(retry()).not.toBeNull()
  expect(submit().disabled).toBe(true)
  expect(c.prevent?.()).toBe(false)
  const tabs = root.querySelectorAll<HTMLButtonElement>('[role="tab"]')
  tabs[1].click()
  tabs[0].click()
  await settle()
  expect(retry()).not.toBeNull()
  retry().click()
  await settle()
  expect(count("prepareInstance")).toHaveLength(2)
  const gate = deferred()
  c.mutate.mockImplementation(() => gate.promise)
  invoke("instance-creation-retry")
  invoke("instance-creation-retry")
  await settle()
  expect(retry().disabled).toBe(true)
  expect(count("prepareInstance")).toHaveLength(3)
  gate.resolve(11)
  await settle()
  expect(count("createInstance")).toHaveLength(1)
  expect(
    c.fetch.mock.calls.filter((x) => x[0].includes("loadIcon"))
  ).toHaveLength(1)
  expect(count("prepareInstance").every((x) => x[1] === 7)).toBe(true)
  expect(c.closeOwn).toHaveBeenCalledTimes(1)
  expect(c.navigate).toHaveBeenCalledTimes(1)
})
test.each(["icon", "create", "prepare"])(
  "disposed %s continuation cannot prepare, close a replacement, navigate or toast",
  async (stage) => {
    const gate = deferred()
    mount()
    await c.picker.onSelect("/synthetic/icon.png")
    if (stage === "icon") c.fetch.mockImplementation(() => gate.promise)
    else
      c.mutate.mockImplementation((key) =>
        key ===
        `instance.${stage === "create" ? "createInstance" : "prepareInstance"}`
          ? gate.promise
          : Promise.resolve(7)
      )
    invoke()
    await settle()
    dispose!()
    dispose = undefined
    gate.resolve(stage === "icon" ? { ok: true } : 7)
    await settle()
    expect(c.closeOwn).not.toHaveBeenCalled()
    expect(c.close).not.toHaveBeenCalled()
    expect(c.navigate).not.toHaveBeenCalled()
    expect(toast.success).not.toHaveBeenCalled()
    expect(toast.error).not.toHaveBeenCalled()
    if (stage !== "prepare") expect(count("prepareInstance")).toHaveLength(0)
  }
)
test("repeated vanilla create while create is pending allocates only one identity", async () => {
  const gate = deferred()
  mount()
  c.mutate.mockImplementation(() => gate.promise)
  invoke()
  invoke()
  await settle()
  expect(count("createInstance")).toHaveLength(1)
  gate.resolve(7)
  await settle()
  expect(count("prepareInstance")).toHaveLength(1)
})

test("a single actual Create click admits one vanilla instance and reports admission", async () => {
  mount()
  submit().click()
  await settle()
  expect(count("createInstance")).toHaveLength(1)
  expect(count("prepareInstance")).toEqual([["instance.prepareInstance", 7]])
  expect(toast.success).toHaveBeenCalledExactlyOnceWith(
    "instances:_trn_custom_create_admitted"
  )
})
