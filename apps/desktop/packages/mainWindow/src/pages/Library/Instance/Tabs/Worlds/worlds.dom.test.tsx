import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import type {
  FEDatapackUpdateResult,
  FEWorld,
  FEWorldDatapack,
  Mod
} from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  state: null as any,
  navigate: vi.fn(),
  openModal: vi.fn(),
  deleteWorld: vi.fn(),
  updateWorld: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  refetchWorlds: vi.fn(),
  queryOptions: {} as Record<string, any>,
  mutate: {} as Record<string, ReturnType<typeof vi.fn>>,
  mutateAsync: {} as Record<string, ReturnType<typeof vi.fn>>,
  clientQuery: vi.fn()
}))

vi.mock("@solidjs/router", () => ({ useParams: () => ({ id: "5" }) }))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: io.navigate })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: io.openModal })
}))
vi.mock("../Addons/hooks/useAddonMutations", () => ({
  useAddonMutations: () => ({
    handleDeleteMod: io.deleteWorld,
    handleUpdateMod: io.updateWorld
  })
}))
vi.mock("@/pages/Library/Instance/instance.data", () => ({
  default: () => ({
    instanceDetails: {
      get data() {
        return io.state.details
      }
    }
  })
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { query: io.clientQuery } }),
    createQuery: (options: any) => {
      const key = options().queryKey[0]
      io.queryOptions[key] = options()
      if (key === "instance.getWorlds")
        return {
          get data() {
            return io.state.worldsError ? undefined : io.state.worlds
          },
          get isLoading() {
            return false
          },
          get isError() {
            return io.state.worldsError
          },
          refetch: io.refetchWorlds
        }
      return {
        get data() {
          return io.state.mods
        },
        isLoading: false,
        isError: false,
        refetch: vi.fn(async () => {})
      }
    },
    createMutation: (options: any) => {
      const key = options().mutationKey[0]
      io.mutate[key] ??= vi.fn()
      io.mutateAsync[key] ??= vi.fn()
      return { mutate: io.mutate[key], mutateAsync: io.mutateAsync[key] }
    }
  }
}))
vi.mock("@gd/i18n", () => {
  const t = (key: string, options?: any) =>
    options?.places ? `${key}(${options.places})` : key
  return {
    Trans: (p: any) => <span>{p.key}</span>,
    useTransContext: () => [t],
    i18n: { t }
  }
})
vi.mock("@/utils/instances", () => ({
  getWorldDatapackImageUrl: () => null
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/uiMenus.fixture")
  return {
    ...ui,
    PRESS_CLASSES: "",
    Skeleton: { logsList: () => <div /> },
    Button: (p: any) => (
      <button
        data-testid={p["data-testid"]}
        disabled={p.disabled}
        onClick={p.onClick}
      >
        {p.children}
      </button>
    ),
    DropdownMenuItem: (p: any) => (
      <button
        data-menu-item
        data-testid={p["data-testid"]}
        disabled={p.disabled}
        onClick={() => p.onSelect?.()}
      >
        {p.children}
      </button>
    ),
    toast: { error: io.toastError, success: io.toastSuccess }
  }
})

import Worlds from "./index"

const modrinth = (project_id: string) => ({
  project_id,
  version_id: "v",
  title: "t",
  version: "1",
  urlslug: "s",
  description: "",
  authors: "",
  has_image: false
})
const curseforge = (project_id: number) => ({
  project_id,
  file_id: 1,
  name: "n",
  version: "1",
  urlslug: "slug",
  summary: "",
  authors: "",
  has_image: false
})
const pack = (
  id: string,
  over: Partial<FEWorldDatapack> = {}
): FEWorldDatapack => ({
  id,
  filename: `${id}.zip`,
  file_size: 1,
  is_folder: false,
  name: null,
  version: null,
  curseforge: null,
  modrinth: null,
  has_update: false,
  ...over
})
const known = (id: string, project: string, has_update = false) =>
  pack(id, { name: `Pack ${id}`, modrinth: modrinth(project), has_update })
const world = (
  name: string,
  datapacks: FEWorldDatapack[],
  identity: string | null = name
): FEWorld => ({ name, addon_id: `addon-${name}`, identity, datapacks })
const worldRow = (name: string, over: Partial<Mod> = {}): Mod => ({
  id: `addon-${name}`,
  filename: name,
  enabled: true,
  addon_type: "worlds",
  metadata: null,
  curseforge: null,
  modrinth: null,
  has_update: false,
  is_duplicate: false,
  file_size: 1,
  ...over
})

let dispose: (() => void) | undefined
let setState: any
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.mutate = {}
  io.mutateAsync = {}
  io.clientQuery.mockResolvedValue({ type: "Succeeded" })
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})

function mount(
  worlds: FEWorld[],
  options: { mods?: Mod[]; details?: any; worldsError?: boolean } = {}
) {
  const [state, set] = createStore({
    worlds,
    mods: options.mods ?? [],
    details: options.details ?? { modpack: null, state: { state: "inactive" } },
    worldsError: options.worldsError ?? false
  })
  io.state = state
  setState = set
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Worlds />, root)
  return root
}
const all = (root: HTMLElement, id: string) => [
  ...root.querySelectorAll<HTMLElement>(`[data-testid="${id}"]`)
]
const one = (root: HTMLElement, id: string) =>
  root.querySelector<HTMLElement>(`[data-testid="${id}"]`)
const title = (root: HTMLElement) => one(root, "world-title")?.textContent
const select = (root: HTMLElement, name: string) =>
  all(root, "world-entry")
    .find((entry) => entry.textContent?.includes(name))!
    .click()

it("selects the first world and asks the core to watch the instance", async () => {
  const root = mount([world("Alpha", []), world("Beta", [])])
  await flush()
  expect(title(root)).toBe("Alpha")
  expect(all(root, "world-entry")[0].dataset.selected).toBe("true")
  expect(io.mutate["instance.prioritizeInstanceCache"]).toHaveBeenCalledWith(5)
})

it("keeps the chosen world across a refetch while it still exists", async () => {
  const root = mount([world("Alpha", []), world("Beta", [])])
  await flush()
  select(root, "Beta")
  await flush()
  expect(title(root)).toBe("Beta")

  setState("worlds", [
    world("Aardvark", []),
    world("Alpha", []),
    world("Beta", [known("p", "terra")])
  ])
  await flush()
  expect(title(root)).toBe("Beta")

  setState("worlds", [world("Aardvark", []), world("Alpha", [])])
  await flush()
  expect(title(root)).toBe("Aardvark")
})

it("shows a known pack's name over its file name, and an unknown one by file name with Remove only", async () => {
  const root = mount([
    world("Alpha", [known("terra", "terra"), pack("better_villages")])
  ])
  await flush()
  const [first, second] = all(root, "world-datapack")
  expect(one(first, "world-datapack-name")!.textContent).toBe("Pack terra")
  expect(one(first, "world-datapack-filename")!.textContent).toBe("terra.zip")
  expect(one(first, "world-datapack-switch")).not.toBeNull()

  expect(one(second, "world-datapack-name")!.textContent).toBe(
    "better_villages.zip"
  )
  expect(one(second, "world-datapack-filename")).toBeNull()
  expect(one(second, "world-datapack-switch")).toBeNull()
  expect(one(second, "world-datapack-update")).toBeNull()
  expect(one(second, "world-datapack-remove")).not.toBeNull()
})

it("updates the only copy with an update straight from its arrow", async () => {
  const root = mount([world("Alpha", [known("terra", "terra", true)])])
  await flush()
  io.mutateAsync["instance.updateWorldDatapack"].mockResolvedValue(9)
  expect(one(root, "world-datapack-update-everywhere")).toBeNull()
  one(root, "world-datapack-update")!.click()
  await flush()
  expect(io.mutateAsync["instance.updateWorldDatapack"]).toHaveBeenCalledWith({
    instance_id: 5,
    id: "terra"
  })
  expect(io.clientQuery).toHaveBeenCalledWith(["vtask.getTaskOutcome", 9])
})

it("offers Update here and Update everywhere for three copies with an update", async () => {
  const root = mount(
    [
      world("Alpha", [known("a", "terra", true)]),
      world("Beta", [known("b", "terra", true)])
    ],
    {
      mods: [
        {
          ...worldRow("x"),
          id: "inst",
          addon_type: "datapacks",
          modrinth: modrinth("terra"),
          has_update: true
        }
      ]
    }
  )
  await flush()
  const everywhere = one(root, "world-datapack-update-everywhere")!
  expect(everywhere.textContent).toContain("3")
  expect(one(root, "world-datapack-update-here")).not.toBeNull()

  io.mutateAsync["instance.updateDatapackEverywhere"].mockResolvedValue([])
  everywhere.click()
  await flush()
  expect(
    io.mutateAsync["instance.updateDatapackEverywhere"]
  ).toHaveBeenCalledWith({ instance_id: 5, origin: { World: "a" } })
  expect(io.mutateAsync["instance.updateWorldDatapack"]).not.toHaveBeenCalled()
  expect(io.toastError).not.toHaveBeenCalled()
})

it("names the places Update everywhere could not update", async () => {
  const root = mount([
    world("Alpha", [known("a", "terra", true)]),
    world("Beta", [known("b", "terra", true)])
  ])
  await flush()
  const results: FEDatapackUpdateResult[] = [
    { place: "Alpha", ok: true, error: null },
    { place: "Beta", ok: false, error: "disk full" },
    { place: null, ok: false, error: "gone" }
  ]
  io.mutateAsync["instance.updateDatapackEverywhere"].mockResolvedValue(results)
  one(root, "world-datapack-update-everywhere")!.click()
  await flush()
  expect(io.toastError).toHaveBeenCalledExactlyOnceWith(
    "content:_trn_worlds.update_failed_in(Beta, content:_trn_worlds.instance_copy)",
    { description: "disk full" }
  )
})

it("removes a pack through deleteWorldDatapack and shows it pending meanwhile", async () => {
  const root = mount([world("Alpha", [pack("loose")])])
  await flush()
  let finish: (value: null) => void = () => {}
  io.mutateAsync["instance.deleteWorldDatapack"].mockReturnValue(
    new Promise((resolve) => (finish = resolve))
  )
  one(root, "world-datapack-remove")!.click()
  await flush()
  expect(io.mutateAsync["instance.deleteWorldDatapack"]).toHaveBeenCalledWith({
    instance_id: 5,
    id: "loose"
  })
  expect(one(root, "world-datapack")!.dataset.pending).toBe("true")
  one(root, "world-datapack-remove")!.click()
  expect(io.mutateAsync["instance.deleteWorldDatapack"]).toHaveBeenCalledTimes(
    1
  )
  finish(null)
  await flush()
  expect(one(root, "world-datapack")!.dataset.pending).toBe("false")
})

it("says the instance has no worlds", async () => {
  const root = mount([])
  await flush()
  expect(one(root, "worlds-none")!.textContent).toBe("content:_trn_worlds.none")
  expect(one(root, "world-title")).toBeNull()
})

it.each([
  ["running", { modpack: null, state: { state: "running", value: {} } }],
  ["queued", { modpack: null, state: { state: "queued", value: 0 } }],
  ["locked", { modpack: { locked: true }, state: { state: "inactive" } }]
])(
  "disables the world's changes while the instance is %s",
  async (_, details) => {
    const root = mount([world("Alpha", [known("a", "terra", true)])], {
      details
    })
    await flush()
    one(root, "world-datapack-update")!.click()
    one(root, "world-datapack-remove")!.click()
    one(root, "world-datapack-switch")!.click()
    one(root, "world-delete")!.click()
    await flush()
    expect(
      (one(root, "world-add-datapacks") as HTMLButtonElement).disabled
    ).toBe(true)
    expect(
      io.mutateAsync["instance.updateWorldDatapack"]
    ).not.toHaveBeenCalled()
    expect(
      io.mutateAsync["instance.deleteWorldDatapack"]
    ).not.toHaveBeenCalled()
    expect(io.navigate).not.toHaveBeenCalled()
    expect(io.deleteWorld).not.toHaveBeenCalled()
    one(root, "world-open-folder")!.click()
    expect(io.mutate["instance.openInstanceFolder"]).toHaveBeenCalledWith({
      instance_id: 5,
      folder: "Saves"
    })
  }
)

it("shows a failed world list in place with a retry, not as a toast", async () => {
  const root = mount([], { worldsError: true })
  await flush()
  expect(io.queryOptions["instance.getWorlds"].meta).toEqual({
    localErrorToast: true
  })
  const failed = one(root, "worlds-load-failed")!
  expect(failed.textContent).toContain("content:_trn_worlds.load_failed")
  expect(one(root, "worlds-none")).toBeNull()
  failed.querySelector("button")!.click()
  expect(io.refetchWorlds).toHaveBeenCalled()
  expect(io.toastError).not.toHaveBeenCalled()
})

it("offers only Open folder and Delete for a world it cannot read", async () => {
  const root = mount([world("Broken", [pack("x")], null)])
  await flush()
  expect(one(root, "world-unreadable")).not.toBeNull()
  expect(one(root, "world-datapack")).toBeNull()
  expect(one(root, "world-add-datapacks")).toBeNull()
  expect(one(root, "world-open-folder")).not.toBeNull()
  expect(one(root, "world-delete")).not.toBeNull()
  expect(one(root, "world-entry")!.textContent).toBe("Broken")
})

it("deletes the world through the addon deletion with its row", async () => {
  const row = worldRow("Alpha")
  const root = mount([world("Alpha", [])], { mods: [row] })
  await flush()
  one(root, "world-delete")!.click()
  expect(io.deleteWorld).toHaveBeenCalledExactlyOnceWith(row)
})

it("opens the search and the versions page for this world", async () => {
  const root = mount([world("My World", [known("a", "terra")])])
  await flush()
  one(root, "world-add-datapacks")!.click()
  expect(io.navigate).toHaveBeenLastCalledWith(
    "/search/datapack?instanceId=5&world=My%20World"
  )
  one(root, "world-datapack-switch")!.click()
  expect(io.navigate).toHaveBeenLastCalledWith(
    "/addon/terra/modrinth/versions?instanceId=5&world=My%20World"
  )
})

it("offers only Copy name in the more menu of a world not downloaded from a platform", async () => {
  const root = mount([world("Alpha", []), world("Beta", [])], {
    mods: [worldRow("Alpha"), worldRow("Beta", { curseforge: curseforge(77) })]
  })
  await flush()
  expect(
    [...root.querySelectorAll("[data-menu-item]")].map((item) =>
      item.getAttribute("data-testid")
    )
  ).toEqual(["world-copy-name"])
  select(root, "Beta")
  await flush()
  one(root, "world-view-details")!.click()
  expect(io.openModal).toHaveBeenCalledWith(
    { name: "modDetails" },
    expect.objectContaining({ instanceId: 5 })
  )
})

it("copies the world's name", async () => {
  const writeText = vi.fn(async () => {})
  vi.stubGlobal("navigator", { ...window.navigator, clipboard: { writeText } })
  try {
    const root = mount([world("My World", [], null)])
    await flush()
    one(root, "world-copy-name")!.click()
    expect(writeText).toHaveBeenCalledExactlyOnceWith("My World")
    expect(io.toastSuccess).toHaveBeenCalled()
  } finally {
    vi.unstubAllGlobals()
  }
})

it("offers Update only for a world with an update, through the addon update", async () => {
  const row = worldRow("Beta", { modrinth: modrinth("w"), has_update: true })
  io.updateWorld.mockResolvedValue(undefined)
  const root = mount([world("Alpha", []), world("Beta", [])], {
    mods: [worldRow("Alpha", { modrinth: modrinth("v") }), row]
  })
  await flush()
  expect(one(root, "world-update")).toBeNull()
  select(root, "Beta")
  await flush()
  one(root, "world-update")!.click()
  expect(io.updateWorld).toHaveBeenCalledExactlyOnceWith(row)
})

it("disables a world's Update while the game runs", async () => {
  const row = worldRow("Alpha", { modrinth: modrinth("w"), has_update: true })
  const root = mount([world("Alpha", [])], {
    mods: [row],
    details: { modpack: null, state: { state: "running", value: {} } }
  })
  await flush()
  const update = one(root, "world-update") as HTMLButtonElement
  expect(update.disabled).toBe(true)
  update.click()
  expect(io.updateWorld).not.toHaveBeenCalled()
})

it("counts each world's data packs and marks the ones with an update", async () => {
  const root = mount([
    world("Alpha", [known("a", "terra", true), pack("b")]),
    world("Beta", [])
  ])
  await flush()
  const [alpha, beta] = all(root, "world-entry")
  expect(alpha.textContent).toContain("content:_trn_worlds.datapack_count")
  expect(one(alpha, "world-entry-update")).not.toBeNull()
  expect(beta.textContent).toContain("content:_trn_worlds.no_datapacks")
  expect(one(beta, "world-entry-update")).toBeNull()
})
