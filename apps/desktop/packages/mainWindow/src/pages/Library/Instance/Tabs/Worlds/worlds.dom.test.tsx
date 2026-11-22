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
      if (key === "settings.getSettings")
        return {
          get data() {
            return io.state.settings
          }
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
  identity: string | null = name,
  titled: string | null = null
): FEWorld => ({
  name,
  title: titled,
  addon_id: `addon-${name}`,
  identity,
  datapacks
})
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

/** `recycleBin`: the "Deletion through Recycle Bin" setting, on unless given;
 *  `null` while the settings have not loaded. */
function mount(
  worlds: FEWorld[],
  options: {
    mods?: Mod[]
    details?: any
    worldsError?: boolean
    recycleBin?: boolean | null
  } = {}
) {
  const [state, set] = createStore({
    worlds,
    mods: options.mods ?? [],
    details: options.details ?? { modpack: null, state: { state: "inactive" } },
    worldsError: options.worldsError ?? false,
    settings:
      options.recycleBin === null
        ? undefined
        : { deletionThroughRecycleBin: options.recycleBin ?? true }
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

it("names a world by its in-game name, with the folder beneath the title only when it differs", async () => {
  const root = mount([
    world("New World (2)", [], "id", "Skyblock with friends"),
    world("Creative", [], "id2")
  ])
  await flush()
  expect(title(root)).toBe("Skyblock with friends")
  expect(one(root, "world-folder")!.textContent).toBe("New World (2)")
  const entries = all(root, "world-entry")
  expect(entries[0].textContent).toContain("Skyblock with friends")
  expect(entries[0].textContent).not.toContain("New World (2)")
  select(root, "Creative")
  await flush()
  expect(title(root)).toBe("Creative")
  expect(one(root, "world-folder")).toBeNull()
})

it("follows the label of two worlds that share it with their folder, in the sidebar", async () => {
  const root = mount([
    world("Survival", [], "a", "Survival"),
    world("Survival (2)", [], "b", "Survival"),
    world("Creative", [], "c")
  ])
  await flush()
  const [first, second, third] = all(root, "world-entry")
  expect(first.textContent).toContain("Survival Survival")
  expect(second.textContent).toContain("Survival Survival (2)")
  expect(third.textContent).toContain("Creative")
  expect(third.textContent).not.toContain("Creative Creative")
  // The selection is still the folder name.
  select(root, "Survival (2)")
  await flush()
  expect(one(root, "world-folder")!.textContent).toBe("Survival (2)")
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

it("names a place Update everywhere could not update by its in-game name", async () => {
  const root = mount([
    world("Alpha", [known("a", "terra", true)], "a", "Skyblock"),
    world("Beta", [known("b", "terra", true)])
  ])
  await flush()
  io.mutateAsync["instance.updateDatapackEverywhere"].mockResolvedValue([
    { place: "Alpha", ok: false, error: "disk full" }
  ] satisfies FEDatapackUpdateResult[])
  one(root, "world-datapack-update-everywhere")!.click()
  await flush()
  expect(io.toastError).toHaveBeenCalledExactlyOnceWith(
    "content:_trn_worlds.update_failed_in(Skyblock)",
    { description: "disk full" }
  )
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

const removalQuestion = () =>
  io.openModal.mock.calls.find(
    ([modal]) => modal.name === "confirmAddonDeletion"
  )?.[1]

it.each([
  ["a zip no platform knows", pack("loose"), false],
  ["a zip no platform knows, before the setting is known", pack("loose"), null],
  ["a folder", pack("folder", { filename: "folder", is_folder: true }), false]
])(
  "asks before removing %s when it would not go to the Recycle Bin",
  async (_, target, recycleBin) => {
    const root = mount([world("Alpha", [target])], { recycleBin })
    await flush()
    io.mutateAsync["instance.deleteWorldDatapack"].mockResolvedValue(null)
    one(root, "world-datapack-remove")!.click()
    await flush()
    expect(
      io.mutateAsync["instance.deleteWorldDatapack"]
    ).not.toHaveBeenCalled()
    const question = removalQuestion()
    expect(question.addons).toEqual([
      { name: target.filename, downloadable: false }
    ])

    question.onConfirm()
    await flush()
    expect(
      io.mutateAsync["instance.deleteWorldDatapack"]
    ).toHaveBeenCalledExactlyOnceWith({ instance_id: 5, id: target.id })
  }
)

it.each([
  ["a zip no platform knows that goes to the Recycle Bin", pack("loose"), true],
  ["a pack that can be downloaded again", known("terra", "terra"), false]
])("removes %s without asking", async (_, target, recycleBin) => {
  const root = mount([world("Alpha", [target])], { recycleBin })
  await flush()
  io.mutateAsync["instance.deleteWorldDatapack"].mockResolvedValue(null)
  one(root, "world-datapack-remove")!.click()
  await flush()
  expect(removalQuestion()).toBeUndefined()
  expect(
    io.mutateAsync["instance.deleteWorldDatapack"]
  ).toHaveBeenCalledExactlyOnceWith({ instance_id: 5, id: target.id })
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
  "disables the data pack changes while the instance is %s",
  async (_, details) => {
    const root = mount([world("Alpha", [known("a", "terra", true)])], {
      details
    })
    await flush()
    one(root, "world-datapack-update")!.click()
    one(root, "world-datapack-remove")!.click()
    one(root, "world-datapack-switch")!.click()
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
    one(root, "world-open-folder")!.click()
    expect(io.mutate["instance.openWorldFolder"]).toHaveBeenCalledWith({
      instance_id: 5,
      world: "Alpha"
    })
  }
)

it.each([
  ["running", { modpack: null, state: { state: "running", value: {} } }],
  ["preparing", { modpack: null, state: { state: "preparing", value: 0 } }],
  ["queued", { modpack: null, state: { state: "queued", value: 0 } }]
])("does not delete a world while the game is %s", async (_, details) => {
  const root = mount([world("Alpha", [])], {
    mods: [worldRow("Alpha")],
    details
  })
  await flush()
  const action = one(root, "world-delete")!
  expect(action.dataset.disabled).toBe("true")
  // The tooltip says why, as the other blocked actions do.
  expect(action.parentElement!.parentElement!.textContent).toBe(
    "content:_trn_worlds.busy"
  )
  action.click()
  expect(io.deleteWorld).not.toHaveBeenCalled()
})

it("does not delete a world of a locked modpack", async () => {
  const root = mount([world("Alpha", [])], {
    mods: [worldRow("Alpha")],
    details: { modpack: { locked: true }, state: { state: "inactive" } }
  })
  await flush()
  const action = one(root, "world-delete")!
  expect(action.dataset.disabled).toBe("true")
  expect(action.parentElement!.parentElement!.textContent).toBe(
    "instances:_trn_locked_cannot_apply_changes"
  )
  action.click()
  expect(io.deleteWorld).not.toHaveBeenCalled()
})

it("deletes a world while the game is not running", async () => {
  const row = worldRow("Alpha")
  const root = mount([world("Alpha", [])], {
    mods: [row],
    details: { modpack: null, state: { state: "inactive" } }
  })
  await flush()
  const action = one(root, "world-delete")!
  expect(action.dataset.disabled).toBe("false")
  expect(action.parentElement!.parentElement!.textContent).toBe(
    "content:_trn_delete_mod"
  )
  action.click()
  expect(io.deleteWorld).toHaveBeenCalledExactlyOnceWith(row, "Alpha")
})

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
  expect(io.deleteWorld).toHaveBeenCalledExactlyOnceWith(row, "Alpha")
})

it("asks about deleting a world by its in-game name", async () => {
  const row = worldRow("New World")
  const root = mount([world("New World", [], "id", "Skyblock")], {
    mods: [row]
  })
  await flush()
  one(root, "world-delete")!.click()
  expect(io.deleteWorld).toHaveBeenCalledExactlyOnceWith(row, "Skyblock")
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

it("opens the versions of a world downloaded from a platform, for the instance and no world", async () => {
  const root = mount([world("Alpha", []), world("Beta", [])], {
    mods: [worldRow("Alpha"), worldRow("Beta", { curseforge: curseforge(77) })],
    // Another version is a save of its own: the game may be running.
    details: { modpack: null, state: { state: "running", value: {} } }
  })
  await flush()
  expect(one(root, "world-switch-version")).toBeNull()
  select(root, "Beta")
  await flush()
  const item = one(root, "world-switch-version")!
  expect(item.textContent).toContain("instances:_trn_switch_version")
  item.click()
  expect(io.navigate).toHaveBeenCalledExactlyOnceWith(
    "/addon/77/curseforge/versions?instanceId=5"
  )
})

it("asks which platform's versions to open for a world both know", async () => {
  const root = mount([world("Alpha", [])], {
    mods: [
      worldRow("Alpha", {
        curseforge: curseforge(77),
        modrinth: modrinth("w")
      })
    ]
  })
  await flush()
  one(root, "world-switch-version")!.click()
  expect(io.navigate).not.toHaveBeenCalled()
  const [modal, data] = io.openModal.mock.calls[0]
  expect(modal).toEqual({ name: "platformSelection" })
  expect(data.modName).toBe("Alpha")
  data.onSelectPlatform("modrinth")
  expect(io.navigate).toHaveBeenCalledExactlyOnceWith(
    "/addon/w/modrinth/versions?instanceId=5"
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
    writeText.mockClear()
    dispose?.()
    dispose = undefined
    const titled = mount([world("New World", [], null, "Skyblock")])
    await flush()
    one(titled, "world-copy-name")!.click()
    expect(writeText).toHaveBeenCalledExactlyOnceWith("Skyblock")
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

const BUSY = { modpack: null, state: { state: "running", value: {} } }
const LOCKED = { modpack: { locked: true }, state: { state: "inactive" } }

const field = (root: HTMLElement) =>
  one(root, "world-rename-input") as HTMLInputElement | null
const typeInto = (input: HTMLInputElement, value: string) => {
  input.value = value
  input.dispatchEvent(new Event("input", { bubbles: true }))
}
const press = (input: HTMLInputElement, key: string) =>
  input.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }))
/** Mounts the world "New World" titled "Skyblock" after "Alpha", selects it
 *  and opens its name field. */
async function renaming(options: { details?: any } = {}) {
  const root = mount(
    [world("Alpha", []), world("New World", [], "id-new", "Skyblock")],
    options
  )
  await flush()
  select(root, "Skyblock")
  await flush()
  one(root, "world-rename")!.click()
  await flush()
  return root
}

it.each([
  ["a readable world", world("Alpha", []), undefined],
  ["a world it cannot read", world("Alpha", [], null), undefined],
  ["a world of a busy instance", world("Alpha", []), BUSY]
])(
  "open folder opens the world's own folder, for %s",
  async (_, w, details) => {
    const root = mount([world("Other", []), w], { details })
    await flush()
    select(root, "Alpha")
    await flush()
    one(root, "world-open-folder")!.click()
    expect(
      io.mutate["instance.openWorldFolder"]
    ).toHaveBeenCalledExactlyOnceWith({ instance_id: 5, world: "Alpha" })
    expect(io.mutate["instance.openInstanceFolder"]).toBeUndefined()
  }
)

it("the pencil turns the title into a field holding the name", async () => {
  const root = await renaming()
  const input = field(root)!
  expect(input.value).toBe("Skyblock")
  expect(input.maxLength).toBe(64)
  expect(document.activeElement).toBe(input)
  expect([input.selectionStart, input.selectionEnd]).toEqual([0, 8])
  expect(one(root, "world-title")).toBeNull()
  expect(one(root, "world-rename-save")).not.toBeNull()
  expect(one(root, "world-rename-cancel")).not.toBeNull()
  // The folder line stays beneath it.
  expect(one(root, "world-folder")!.textContent).toBe("New World")
})

it("Enter saves the typed name and the renamed world stays selected", async () => {
  const root = await renaming()
  io.mutateAsync["instance.renameWorld"].mockResolvedValue("Season 2")
  io.refetchWorlds.mockImplementation(async () =>
    setState("worlds", [
      world("Alpha", []),
      world("Season 2", [], "id-season", "Season 2")
    ])
  )
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Enter")
  await flush()
  expect(
    io.mutateAsync["instance.renameWorld"]
  ).toHaveBeenCalledExactlyOnceWith({
    instance_id: 5,
    world: "New World",
    identity: "id-new",
    name: "Season 2"
  })
  expect(io.refetchWorlds).toHaveBeenCalled()
  expect(field(root)).toBeNull()
  expect(title(root)).toBe("Season 2")
  expect(one(root, "world-folder")).toBeNull()
  const selected = all(root, "world-entry").find(
    (entry) => entry.dataset.selected === "true"
  )
  expect(selected!.textContent).toContain("Season 2")
})

it("the tick saves, the cross and Escape cancel without a request", async () => {
  const root = await renaming()
  io.mutateAsync["instance.renameWorld"].mockResolvedValue("New World")
  typeInto(field(root)!, "Season 2")
  one(root, "world-rename-cancel")!.click()
  await flush()
  expect(field(root)).toBeNull()
  expect(title(root)).toBe("Skyblock")

  // Escape cancels and reaches nothing around the field.
  const outer = vi.fn()
  document.body.addEventListener("keydown", outer)
  one(root, "world-rename")!.click()
  await flush()
  expect(field(root)!.value).toBe("Skyblock")
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Escape")
  await flush()
  document.body.removeEventListener("keydown", outer)
  expect(field(root)).toBeNull()
  expect(outer).not.toHaveBeenCalled()

  // A click outside the field cancels too.
  one(root, "world-rename")!.click()
  await flush()
  typeInto(field(root)!, "Season 2")
  document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }))
  await flush()
  expect(field(root)).toBeNull()
  expect(io.mutateAsync["instance.renameWorld"]).not.toHaveBeenCalled()

  one(root, "world-rename")!.click()
  await flush()
  typeInto(field(root)!, "Season 2")
  one(root, "world-rename-save")!.click()
  await flush()
  expect(
    io.mutateAsync["instance.renameWorld"]
  ).toHaveBeenCalledExactlyOnceWith({
    instance_id: 5,
    world: "New World",
    identity: "id-new",
    name: "Season 2"
  })
  expect(field(root)).toBeNull()
})

it.each([["   "], [" Skyblock "]])(
  "a blank or unchanged name sends nothing (%j)",
  async (typed) => {
    const root = await renaming()
    typeInto(field(root)!, typed)
    press(field(root)!, "Enter")
    await flush()
    expect(field(root)).toBeNull()
    expect(io.mutateAsync["instance.renameWorld"]).not.toHaveBeenCalled()
  }
)

it("the field is disabled while the rename runs", async () => {
  const root = await renaming()
  let answer!: (_folder: string) => void
  io.mutateAsync["instance.renameWorld"].mockReturnValue(
    new Promise<string>((resolve) => (answer = resolve))
  )
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Enter")
  await flush()
  expect(field(root)!.disabled).toBe(true)
  expect(one(root, "world-rename-save")!.dataset.disabled).toBe("true")
  expect(one(root, "world-rename-cancel")!.dataset.disabled).toBe("true")
  one(root, "world-rename-save")!.click()
  one(root, "world-rename-cancel")!.click()
  document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }))
  await flush()
  expect(field(root)).not.toBeNull()
  expect(io.mutateAsync["instance.renameWorld"]).toHaveBeenCalledTimes(1)
  answer("New World")
  await flush()
  expect(field(root)).toBeNull()
})

it("a refused rename keeps the field and the typed text", async () => {
  const root = await renaming()
  io.mutateAsync["instance.renameWorld"].mockRejectedValue(
    new Error("WORLD_NOT_RENAMED")
  )
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Enter")
  await flush()
  expect(field(root)!.value).toBe("Season 2")
  expect(field(root)!.disabled).toBe(false)
  // The global handler tells the refusal.
  expect(io.toastError).not.toHaveBeenCalled()
})

it("selecting another world closes an open name field", async () => {
  const root = await renaming()
  typeInto(field(root)!, "Season 2")
  select(root, "Alpha")
  await flush()
  expect(field(root)).toBeNull()
  expect(title(root)).toBe("Alpha")
})

const selectedEntry = (root: HTMLElement) =>
  all(root, "world-entry").find((entry) => entry.dataset.selected === "true")
    ?.textContent
/** Renames "New World" to "Season 2" and leaves the request running; the
 *  returned `answer` settles it, and the refetch then lists "Season 2". */
async function renameRunning() {
  const root = await renaming()
  let answer!: (_folder: string) => void
  io.mutateAsync["instance.renameWorld"].mockReturnValue(
    new Promise<string>((resolve) => (answer = resolve))
  )
  io.refetchWorlds.mockImplementation(async () =>
    setState("worlds", [
      world("Alpha", []),
      world("Season 2", [], "id-new", "Season 2")
    ])
  )
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Enter")
  await flush()
  return { root, answer: (folder: string) => answer(folder) }
}

it("a rename still running does not disable or close another world's field", async () => {
  const { root, answer } = await renameRunning()
  select(root, "Alpha")
  await flush()
  expect(field(root)).toBeNull()
  one(root, "world-rename")!.click()
  await flush()
  expect(field(root)!.disabled).toBe(false)
  expect(field(root)!.value).toBe("Alpha")
  expect(one(root, "world-rename-save")!.dataset.disabled).toBe("false")
  expect(one(root, "world-rename-cancel")!.dataset.disabled).toBe("false")
  typeInto(field(root)!, "Alpha 2")

  answer("Season 2")
  await flush()
  expect(io.refetchWorlds).toHaveBeenCalled()
  expect(field(root)!.value).toBe("Alpha 2")
  expect(field(root)!.disabled).toBe(false)
  expect(selectedEntry(root)).toContain("Alpha")
})

it("a rename that answers after the player selected another world leaves that selection", async () => {
  const { root, answer } = await renameRunning()
  select(root, "Alpha")
  await flush()
  answer("Season 2")
  await flush()
  expect(io.refetchWorlds).toHaveBeenCalled()
  expect(title(root)).toBe("Alpha")
  expect(selectedEntry(root)).toContain("Alpha")
})

it("the renamed world stays in the header while the lists are read again", async () => {
  const root = await renaming()
  const header = () => title(root) ?? field(root)?.value
  const seen: (string | undefined)[] = []
  io.mutateAsync["instance.renameWorld"].mockImplementation(async () => {
    seen.push(header())
    return "Season 2"
  })
  io.refetchWorlds.mockImplementation(async () => {
    seen.push(header())
    setState("worlds", [
      world("Alpha", []),
      world("Season 2", [], "id-new", "Season 2")
    ])
    seen.push(header())
    await Promise.resolve()
    seen.push(header())
  })
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Enter")
  await flush()
  seen.push(header())
  expect(seen).toEqual([
    "Season 2",
    "Season 2",
    "Season 2",
    "Season 2",
    "Season 2"
  ])
  expect(selectedEntry(root)).toContain("Season 2")
})

it.each([
  ["isComposing", { isComposing: true }],
  ["key code 229", { keyCode: 229 }]
])(
  "an Enter that confirms a composition does not save (%s)",
  async (_, init) => {
    const root = await renaming()
    io.mutateAsync["instance.renameWorld"].mockResolvedValue("Season 2")
    typeInto(field(root)!, "Season 2")
    field(root)!.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true, ...init })
    )
    await flush()
    expect(io.mutateAsync["instance.renameWorld"]).not.toHaveBeenCalled()
    expect(field(root)!.value).toBe("Season 2")
  }
)

it("Enter pressed twice quickly sends one request", async () => {
  const root = await renaming()
  io.mutateAsync["instance.renameWorld"].mockResolvedValue("Season 2")
  typeInto(field(root)!, "Season 2")
  press(field(root)!, "Enter")
  press(field(root)!, "Enter")
  await flush()
  expect(io.mutateAsync["instance.renameWorld"]).toHaveBeenCalledTimes(1)
})

it("a refetch that returns the same world keeps the open field and the typed text", async () => {
  const root = await renaming()
  typeInto(field(root)!, "Season 2")
  setState("worlds", [
    world("Alpha", []),
    world("New World", [known("p", "terra")], "id-new", "Skyblock")
  ])
  await flush()
  expect(field(root)!.value).toBe("Season 2")
  expect(field(root)!.disabled).toBe(false)
})

it.each([
  [
    "the world can no longer be read",
    () =>
      setState("worlds", [
        world("Alpha", []),
        world("New World", [], null, "Skyblock")
      ])
  ],
  ["the game started", () => setState("details", BUSY)]
])(
  "a name typed before %s sends nothing and keeps the field",
  async (_, change) => {
    const root = await renaming()
    typeInto(field(root)!, "Season 2")
    change()
    await flush()
    press(field(root)!, "Enter")
    await flush()
    expect(io.mutateAsync["instance.renameWorld"]).not.toHaveBeenCalled()
    expect(field(root)!.value).toBe("Season 2")
  }
)

it.each([
  [
    "while the game is busy",
    world("Alpha", []),
    BUSY,
    "content:_trn_worlds.busy"
  ],
  [
    "on a locked instance",
    world("Alpha", []),
    LOCKED,
    "instances:_trn_locked_cannot_apply_changes"
  ],
  [
    "for a world that cannot be read",
    world("Alpha", [], null),
    undefined,
    "content:_trn_worlds.unreadable"
  ]
])("rename is disabled with the reason %s", async (_, w, details, reason) => {
  const root = mount([w], { details })
  await flush()
  const pencil = one(root, "world-rename")!
  expect(pencil.dataset.disabled).toBe("true")
  expect(pencil.parentElement!.parentElement!.textContent).toBe(reason)
  pencil.click()
  await flush()
  expect(field(root)).toBeNull()
  expect(io.mutateAsync["instance.renameWorld"]).not.toHaveBeenCalled()
})

it("offers Rename as the pencil's tooltip", async () => {
  const root = mount([world("Alpha", [])])
  await flush()
  const pencil = one(root, "world-rename")!
  expect(pencil.dataset.disabled).toBe("false")
  expect(pencil.parentElement!.parentElement!.textContent).toBe(
    "content:_trn_worlds.rename"
  )
})
