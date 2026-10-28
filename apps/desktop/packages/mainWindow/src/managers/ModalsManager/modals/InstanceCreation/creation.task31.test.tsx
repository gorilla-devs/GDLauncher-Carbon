import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import { input, settle, toast } from "@/task31.fixture"
import InstanceCreation from "."

const c = vi.hoisted(() => ({
  store: undefined as any,
  queries: {} as any,
  mutate: vi.fn(),
  close: vi.fn(),
  navigate: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task31.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.store
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close }),
  useModalStackEntry: () => ({ closeOwn: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div data-title={p.title}>{p.children}</div>
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@/components/ImagePicker", () => ({
  default: () => <span>image picker</span>
}))
vi.mock("./Import", () => ({ default: () => <span>import fixture</span> }))
vi.mock("./ShareCodeImport", () => ({
  default: () => <span>share fixture</span>
}))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "icon" }))
vi.mock("@/utils/helpers", () => ({
  getModloaderDisplayName: (v: string) => v,
  blobToBase64: vi.fn()
}))
vi.mock("@/utils/rspcClient", () => ({
  apiFetch: vi.fn(),
  extractErrorDisplay: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
  rspc: {
    createQuery: (f: any) => c.queries[f().queryKey[0]],
    createMutation: (f: any) => ({
      mutateAsync: (v: any) => c.mutate(f().mutationKey[0], v)
    })
  }
}))

let dispose: (() => void) | undefined, setStore: any, setQueries: any
const release = (id: string, type = "release") => ({
  id,
  type,
  releaseTime: "2020-01-01",
  time: "2020-01-01",
  url: "synthetic.invalid"
})
const manifest = (kind: string) => ({
  gameVersions: [
    {
      id: "1.20.1",
      loaders: [{ id: kind === "neoforge" ? "47.1" : "1.20.1-47.3.0" }]
    },
    {
      id: "1.21.1",
      loaders: [{ id: kind === "neoforge" ? "21.1" : "1.21.1-52.0.0" }]
    }
  ]
})
beforeEach(() => {
  vi.clearAllMocks()
  c.mutate.mockReset()
  c.mutate.mockImplementation(async (key) =>
    key === "instance.createInstance" ? 44 : undefined
  )
  const [store, set] = createStore({
    minecraftVersions: { data: [release("1.21.1"), release("1.20.1")] }
  })
  c.store = store
  setStore = set
  const [queries, setQ] = createStore(
    Object.fromEntries(
      [
        "mc.getForgeVersions",
        "mc.getNeoforgeVersions",
        "mc.getFabricVersions",
        "mc.getQuiltVersions"
      ]
        .map((key) => [
          key,
          {
            data: manifest(key.includes("Neoforge") ? "neoforge" : "forge"),
            isFetched: true,
            isFetching: false,
            isError: false,
            refetch: vi.fn(async () => undefined)
          }
        ])
        .concat([["instance.getDefaultGroup", { data: 3 }]] as any)
    )
  )
  c.queries = queries
  setQueries = setQ
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount(data?: any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <InstanceCreation title="Synthetic creation" data={data} />,
    root
  )
  return root
}
const submit = (root: HTMLElement) =>
  root.querySelector<HTMLButtonElement>(
    '[data-testid="instance-creation-submit"]'
  )!
const chooseGame = (root: HTMLElement, value: string) =>
  (root.querySelector("select") as any).choose(value)
const edit = (kind?: string) => ({
  id: "9",
  title: "Original name",
  mcVersion: "1.20.1",
  modloader: kind,
  modloaderVersion:
    kind === "neoforge" ? "47.1" : kind ? "1.20.1-47.3.0" : undefined,
  img: null
})
it.each([undefined, { groupId: 7 }])(
  "0033 %j context creates and preserves group/default",
  async (data) => {
    const root = mount(data)
    input(root, 0, "Created name")
    expect(submit(root).textContent).toContain("create")
    submit(root).click()
    await settle()
    expect(c.mutate).toHaveBeenCalledWith(
      "instance.createInstance",
      expect.objectContaining({
        group: data?.groupId ?? 3,
        name: "Created name"
      })
    )
    expect(c.mutate).toHaveBeenCalledWith("instance.prepareInstance", 44)
  }
)
it("0033 Import→Custom creates after selecting the actual custom tab", async () => {
  const root = mount({ import: true, groupId: 7 })
  expect(root.textContent).toContain("import fixture")
  root
    .querySelector<HTMLButtonElement>(
      '[data-testid="instance-creation-custom-tab"]'
    )!
    .click()
  input(root, 0, "From import")
  submit(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.createInstance",
    expect.objectContaining({ group: 7, name: "From import" })
  )
})
it.each([9, "9", 0, "0"])(
  "0033 exact numeric/string edit id %j updates that id",
  async (id) => {
    const root = mount({ ...edit(), id })
    input(root, 0, "Renamed instance")
    expect(submit(root).textContent).toContain("update")
    submit(root).click()
    await settle()
    expect(c.mutate).toHaveBeenCalledWith(
      "instance.updateInstance",
      expect.objectContaining({
        instance: Number(id),
        name: { Set: "Renamed instance" }
      })
    )
    expect(
      c.mutate.mock.calls.some(([key]) => key === "instance.createInstance")
    ).toBe(false)
  }
)
it.each(["9junk", "9.5", -1, NaN])(
  "0033 malformed purported edit id %j dispatches neither update nor create",
  async (id) => {
    const root = mount({ ...edit(), id })
    input(root, 0, "Renamed instance")
    ;(submit(root) as any).invoke()
    await settle()
    expect(c.mutate).not.toHaveBeenCalled()
  }
)
it.each(["snapshot", "old_alpha", "old_beta"])(
  "0104 original %s survives hidden filters/refetch and rename only changes name",
  async (type) => {
    setStore("minecraftVersions", "data", [
      release("1.21.1"),
      release("original", type)
    ])
    const root = mount({ ...edit(), mcVersion: "original" })
    expect(root.querySelector("select")?.getAttribute("data-value")).toBe(
      "original"
    )
    setStore("minecraftVersions", "data", [
      release("1.21.1"),
      release("original", type)
    ])
    input(root, 0, "Only rename")
    submit(root).click()
    await settle()
    expect(c.mutate).toHaveBeenCalledWith(
      "instance.updateInstance",
      expect.objectContaining({
        name: { Set: "Only rename" },
        version: null,
        modloader: null
      })
    )
  }
)
it("0104 original release survives pending and missing refetched manifest", async () => {
  setStore("minecraftVersions", "data", undefined as any)
  const root = mount(edit())
  setStore("minecraftVersions", "data", [release("1.21.1")])
  expect(root.querySelector("select")?.getAttribute("data-value")).toBe(
    "1.20.1"
  )
  input(root, 0, "Offline rename")
  submit(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.updateInstance",
    expect.objectContaining({ version: null })
  )
})
it.each(["forge", "neoforge"])(
  "0103 changing game publishes a coherent compatible %s pair",
  async (kind) => {
    const root = mount(edit(kind))
    chooseGame(root, "1.21.1")
    submit(root).click()
    await settle()
    expect(c.mutate).toHaveBeenCalledWith(
      "instance.updateInstance",
      expect.objectContaining({
        version: { Set: "1.21.1" },
        modloader: {
          Set: {
            type_: kind,
            version: kind === "forge" ? "1.21.1-52.0.0" : "21.1"
          }
        }
      })
    )
  }
)
it("0103 explicit compatible loader choice remains supported", async () => {
  setQueries("mc.getForgeVersions", "data", "gameVersions", 1, "loaders", [
    { id: "1.21.1-52.0.0" },
    { id: "1.21.1-52.1.0" }
  ])
  const root = mount(edit("forge"))
  chooseGame(root, "1.21.1")
  ;(root.querySelectorAll("select")[1] as any).choose("1.21.1-52.1.0")
  submit(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.updateInstance",
    expect.objectContaining({
      modloader: { Set: { type_: "forge", version: "1.21.1-52.1.0" } }
    })
  )
})
it("0103 empty/pending/error metadata blocks a changed pair at handler but permits unchanged offline rename", async () => {
  const root = mount(edit("forge"))
  setQueries("mc.getForgeVersions", "data", undefined)
  setQueries("mc.getForgeVersions", "isError", true)
  input(root, 0, "Offline rename")
  ;(submit(root) as any).invoke()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.updateInstance",
    expect.objectContaining({ version: null, modloader: null })
  )
  dispose?.()
  c.mutate.mockClear()
  const other = mount(edit("forge"))
  chooseGame(other, "1.21.1")
  ;(submit(other) as any).invoke()
  await settle()
  expect(c.mutate).not.toHaveBeenCalled()
})
it("0103 handler rejects selected game/loader not belonging to current metadata", async () => {
  const root = mount(edit("forge"))
  chooseGame(root, "1.21.1")
  ;(root.querySelectorAll("select")[1] as any).choose("1.20.1-47.3.0")
  ;(submit(root) as any).invoke()
  await settle()
  expect(c.mutate).not.toHaveBeenCalled()
})
it("0104 new vanilla creation with empty manifest has no unsafe first-element access", () => {
  setStore("minecraftVersions", "data", [])
  let root: HTMLElement | undefined
  expect(() => {
    root = mount()
  }).not.toThrow()
  expect(submit(root!).disabled).toBe(true)
})

it.each(["fabric", "quilt"])(
  "0103 %s uses supported game plus dummy shared loader policy",
  async (kind) => {
    const query =
      kind === "fabric" ? "mc.getFabricVersions" : "mc.getQuiltVersions"
    setQueries(query, "data", {
      gameVersions: [
        { id: "1.20.1", loaders: [] },
        { id: "1.21.1", loaders: [] },
        { id: "${gdlauncher.gameVersion}", loaders: [{ id: "shared-loader" }] }
      ]
    })
    const root = mount({ ...edit(kind), modloaderVersion: "shared-loader" })
    chooseGame(root, "1.21.1")
    submit(root).click()
    await settle()
    expect(c.mutate).toHaveBeenCalledWith(
      "instance.updateInstance",
      expect.objectContaining({
        version: { Set: "1.21.1" },
        modloader: { Set: { type_: kind, version: "shared-loader" } }
      })
    )
  }
)
it("0103 pending metadata preserves deliberate compatible loader choice then validates settled pair", async () => {
  setQueries("mc.getForgeVersions", "data", "gameVersions", 1, "loaders", [
    { id: "1.21.1-52.0.0" },
    { id: "1.21.1-52.1.0" }
  ])
  const root = mount(edit("forge"))
  chooseGame(root, "1.21.1")
  ;(root.querySelectorAll("select")[1] as any).choose("1.21.1-52.1.0")
  setQueries("mc.getForgeVersions", "isFetching", true)
  expect(submit(root).disabled).toBe(true)
  setQueries("mc.getForgeVersions", "isFetching", false)
  expect(root.querySelectorAll("select")[1].getAttribute("data-value")).toBe(
    "1.21.1-52.1.0"
  )
  submit(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.updateInstance",
    expect.objectContaining({
      modloader: { Set: { type_: "forge", version: "1.21.1-52.1.0" } }
    })
  )
})

it("a refused version change keeps the dialog and the choice, and shows the core's reason", async () => {
  const reason =
    "This instance has an unfinished modpack update or repair. Press Play to finish it, or cancel it from Repair, before changing the game version or loader."
  c.mutate.mockImplementation(async (key) => {
    if (key === "instance.updateInstance") throw new Error(reason)
  })
  const root = mount(edit("forge"))
  chooseGame(root, "1.21.1")
  submit(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.updateInstance",
    expect.objectContaining({ version: { Set: "1.21.1" } })
  )
  expect(root.querySelector('[role="alert"]')?.textContent).toBe(reason)
  expect(c.close).not.toHaveBeenCalled()
  expect(toast.error).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  expect(root.querySelector("select")?.getAttribute("data-value")).toBe(
    "1.21.1"
  )
})
it("a failed edit without a reason falls back to a translated message", async () => {
  c.mutate.mockImplementation(async (key) => {
    if (key === "instance.updateInstance") throw new Error("  ")
  })
  const root = mount(edit())
  input(root, 0, "Renamed instance")
  submit(root).click()
  await settle()
  expect(root.querySelector('[role="alert"]')?.textContent).toBe(
    "instances:_trn_custom_update_failed"
  )
  expect(c.close).not.toHaveBeenCalled()
})
it("a successful edit still closes the dialog", async () => {
  const root = mount(edit())
  input(root, 0, "Renamed instance")
  submit(root).click()
  await settle()
  expect(c.close).toHaveBeenCalled()
  expect(toast.success).toHaveBeenCalled()
  expect(root.querySelector('[role="alert"]')).toBeNull()
})
