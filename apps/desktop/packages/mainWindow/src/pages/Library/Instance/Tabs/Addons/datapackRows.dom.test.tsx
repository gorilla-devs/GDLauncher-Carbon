import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import type { FEWorld, FEWorldDatapack, Mod } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  state: null as any,
  openModal: vi.fn(),
  closeModal: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  refetchWorlds: vi.fn(),
  clientQuery: vi.fn(),
  mutateAsync: {} as Record<string, ReturnType<typeof vi.fn>>
}))

vi.mock("@solidjs/router", () => ({ useParams: () => ({ id: "5" }) }))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: io.openModal, closeModal: io.closeModal })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => (
    <div>
      <h1>{p.title}</h1>
      {p.children}
    </div>
  )
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
  queryClient: { ensureQueryData: vi.fn() },
  rspc: {
    useContext: () => ({ client: { query: io.clientQuery } }),
    createQuery: (options: any) => {
      const key = options().queryKey[0]
      if (key === "instance.getInstanceMods")
        return {
          get data() {
            return io.state.rows
          },
          isLoading: false,
          refetch: vi.fn(async () => {})
        }
      if (key === "instance.getWorlds")
        return {
          get data() {
            return io.state.worlds
          },
          isLoading: false,
          isError: false,
          refetch: io.refetchWorlds
        }
      if (key === "server.getLegacyDatapacks") return { data: undefined }
      return { data: undefined, isLoading: false }
    },
    createMutation: (options: any) => {
      const option = options().mutationKey
      const key = Array.isArray(option) ? option[0] : option
      io.mutateAsync[key] ??= vi.fn(async () => undefined)
      return { mutate: vi.fn(), mutateAsync: io.mutateAsync[key] }
    }
  }
}))
vi.mock("@gd/i18n", () => {
  // The key, then the values it names, so a test can read what was said.
  const t = (key: string, options?: Record<string, unknown>) => {
    const values = Object.entries(options ?? {})
      .filter(([name]) => name !== "interpolation")
      .map(([name, value]) => `${name}=${String(value)}`)
    return values.length > 0 ? `${key}(${values.join(";")})` : key
  }
  return {
    Trans: (p: any) => <span>{p.key}</span>,
    useTransContext: () => [t],
    i18n: { t }
  }
})
vi.mock("@/utils/instances", () => ({
  getModImageUrl: () => null,
  getServerModImageUrl: () => null
}))
vi.mock("@/components/CopyIcon", () => ({ default: () => null }))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => <span />
}))
// The cells of the columns these tests read, one row after another.
vi.mock("@/pages/Library/shared/addons/components/AddonTable", async () => {
  const { For } = await import("solid-js")
  const shown = ["filename", "update", "actions"]
  return {
    AddonTable: (p: any) => (
      <For each={p.data()}>
        {(row: any) => (
          <div data-testid="row" data-id={row.id}>
            <For
              each={p.columns.filter((column: any) =>
                shown.includes(column.id ?? column.accessorKey)
              )}
            >
              {(column: any) => column.cell({ row: { original: row } })}
            </For>
          </div>
        )}
      </For>
    )
  }
})
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/uiMenus.fixture")
  return {
    ...ui,
    ...(await import("../../../../../../../../../../packages/ui/src/Button")),
    ...(await import("../../../../../../../../../../packages/ui/src/Badge")),
    ...(await import("../../../../../../../../../../packages/ui/src/Input")),
    Select: () => <span />,
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

import Addons from "./index"
import ServerAddons from "@/pages/Library/Server/Tabs/Addons"
import ConfirmChoice from "@/managers/ModalsManager/modals/ConfirmChoice"

vi.mock("@/pages/Library/Server/Tabs/Addons/hooks", async () => {
  const { createSignal } = await import("solid-js")
  return {
    useServerAddonData: () => {
      const [rowSelection, setRowSelection] = createSignal({})
      return {
        allAddons: { isLoading: false, refetch: vi.fn() },
        addonsStore: io.state.serverRows,
        filteredAddons: () => io.state.serverRows,
        searchQuery: () => "",
        setSearchQuery: vi.fn(),
        enabledAddonTypes: { datapacks: true },
        setEnabledAddonTypes: vi.fn(),
        rowSelection,
        setRowSelection,
        sorting: () => [],
        setSorting: vi.fn(),
        columnFilters: () => [],
        setColumnFilters: vi.fn(),
        columnVisibility: () => ({}),
        setColumnVisibility: vi.fn(),
        optimisticToggleAddon: vi.fn(),
        optimisticDeleteAddon: vi.fn(),
        optimisticDeleteAddons: vi.fn(),
        rollbackToServerState: vi.fn()
      }
    },
    useServerAddonMutations: () => ({
      handleToggleMod: vi.fn(),
      handleDeleteMod: vi.fn(),
      handleDeleteSelected: vi.fn(),
      handleOpenFolder: vi.fn(),
      gotoSearchPage: vi.fn()
    })
  }
})

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
const row = (id: string, over: Partial<Mod> = {}): Mod => ({
  id,
  filename: `${id}.zip`,
  enabled: true,
  addon_type: "datapacks",
  metadata: null,
  curseforge: null,
  modrinth: null,
  has_update: false,
  is_duplicate: false,
  file_size: 1,
  ...over
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
const world = (
  name: string,
  datapacks: FEWorldDatapack[] = [],
  identity: string | null = `id-${name}`,
  title: string | null = null
): FEWorld => ({
  name,
  title,
  addon_id: `addon-${name}`,
  identity,
  datapacks
})
const coded = (code: string, data: unknown) =>
  new Error(
    JSON.stringify({ cause: [{ display: "Refused by the core", code, data }] })
  )

let dispose: (() => void) | undefined
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.mutateAsync = {}
  io.clientQuery.mockResolvedValue({ type: "Succeeded" })
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    }
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.unstubAllGlobals()
})

function mount(
  rows: Mod[],
  worlds: FEWorld[] = [],
  details: any = {
    modloaders: [{}],
    modpack: null,
    state: { state: "inactive" }
  }
) {
  const [state] = createStore({ rows, worlds, details, serverRows: [] })
  io.state = state
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Addons />, root)
  return root
}
const rowOf = (root: HTMLElement, id: string) =>
  root.querySelector<HTMLElement>(`[data-testid="row"][data-id="${id}"]`)!
const within = (element: HTMLElement, testId: string) => [
  ...element.querySelectorAll<HTMLElement>(`[data-testid="${testId}"]`)
]
const mutation = (key: string) => {
  io.mutateAsync[key] ??= vi.fn(async () => undefined)
  return io.mutateAsync[key]
}

it("never lists or counts a world the core returns with the addons", async () => {
  const root = mount([
    row("sodium", { addon_type: "mods", filename: "sodium.jar" }),
    row("My World", { addon_type: "worlds", has_update: true })
  ])
  await flush()
  expect(within(root, "row").map((element) => element.dataset.id)).toEqual([
    "sodium"
  ])
  // The world's update is not counted for "Update all".
  expect(root.textContent).not.toContain("content:_trn_update_all_count")
})

it("names the worlds holding the same project under a data pack, in the list's order", async () => {
  const root = mount(
    [
      row("terralith", {
        modrinth: modrinth("TL"),
        curseforge: curseforge(7)
      }),
      row("local")
    ],
    [
      world("Beta", [pack("b1", { modrinth: modrinth("TL") })]),
      world("Alpha", [pack("a1")]),
      world("Gamma", [pack("g1", { curseforge: curseforge(7) })])
    ]
  )
  await flush()
  // The name cell comes first: its name, then what is under it.
  const nameCell = (id: string) => rowOf(root, id).firstElementChild!
  expect(nameCell("terralith").textContent).toContain(
    "terralith.zipBeta, Gamma"
  )
  // A pack without project ids matches nothing: the row keeps its subtitle.
  expect(nameCell("local").textContent).toContain(
    "local.zipcontent:_trn_datapack_row_subtitle(filename=local.zip)"
  )
  expect(nameCell("local").textContent).not.toContain("Alpha")
})

it("names the worlds of a data pack and of the Add to a world menu by their in-game names, and sends the folder", async () => {
  mutation("instance.copyDatapack").mockResolvedValue("New World")
  const root = mount(
    [row("terralith", { modrinth: modrinth("TL") })],
    [
      world(
        "New World",
        [pack("a", { modrinth: modrinth("TL") })],
        "id-a",
        "Skyblock"
      ),
      world("Survival", [], "id-b", "Survival"),
      world("Survival (2)", [], "id-c", "Survival")
    ]
  )
  await flush()
  expect(rowOf(root, "terralith").firstElementChild!.textContent).toContain(
    "terralith.zipSkyblock"
  )
  const items = within(rowOf(root, "terralith"), "addon-add-to-world-item")
  expect(items.map((item) => item.textContent)).toEqual([
    "Skyblock",
    "Survival Survival",
    "Survival Survival (2)"
  ])
  items[0].click()
  await flush()
  expect(mutation("instance.copyDatapack").mock.calls[0][0].world).toEqual({
    name: "New World",
    identity: "id-a"
  })
  // The core answers with the folder it copied into.
  expect(io.toastSuccess).toHaveBeenCalledExactlyOnceWith(
    "content:_trn_datapack_staging.added(name=terralith.zip;world=Skyblock)"
  )
})

it("asks about replacing in a world by its in-game name", async () => {
  mutation("instance.copyDatapack").mockRejectedValueOnce(
    coded("ALREADY_IN_WORLD", { filenames: ["old.zip"] })
  )
  const root = mount(
    [row("terralith")],
    [world("New World", [], "id-a", "Skyblock")]
  )
  await flush()
  within(rowOf(root, "terralith"), "addon-add-to-world-item")[0].click()
  await flush()
  expect(io.openModal.mock.calls[0][1].title).toBe(
    "content:_trn_worlds.replace_title(world=Skyblock)"
  )
})

it("offers Add to a world on .zip data packs only", async () => {
  const root = mount([
    row("zipped"),
    row("folder", { filename: "folder" }),
    row("off", { filename: "off.zip.disabled", enabled: false }),
    row("sodium", { addon_type: "mods", filename: "sodium.zip" })
  ])
  await flush()
  expect(within(rowOf(root, "zipped"), "addon-add-to-world")).toHaveLength(1)
  for (const id of ["folder", "off", "sodium"])
    expect(within(rowOf(root, id), "addon-add-to-world")).toHaveLength(0)
})

it("lists the readable worlds and copies the pack into the one chosen", async () => {
  mutation("instance.copyDatapack").mockResolvedValue("Beta")
  const root = mount(
    [row("terralith")],
    [world("Alpha"), world("Broken", [], null), world("Beta")]
  )
  await flush()
  const items = within(rowOf(root, "terralith"), "addon-add-to-world-item")
  expect(items.map((item) => item.textContent)).toEqual(["Alpha", "Beta"])
  items[1].click()
  await flush()
  expect(mutation("instance.copyDatapack")).toHaveBeenCalledExactlyOnceWith({
    instance_id: 5,
    mod_id: "terralith",
    world: { name: "Beta", identity: "id-Beta" }
  })
  expect(io.toastSuccess).toHaveBeenCalledExactlyOnceWith(
    "content:_trn_datapack_staging.added(name=terralith.zip;world=Beta)"
  )
  expect(io.refetchWorlds).toHaveBeenCalled()
})

it("says the instance has no worlds when none can take the pack", async () => {
  const root = mount([row("terralith")], [world("Broken", [], null)])
  await flush()
  const element = rowOf(root, "terralith")
  expect(within(element, "addon-add-to-world-item")).toHaveLength(0)
  expect(within(element, "addon-add-to-world-empty")[0].textContent).toBe(
    "content:_trn_worlds.none"
  )
})

it("asks once before replacing the world's copies, then copies with replace", async () => {
  const copy = mutation("instance.copyDatapack")
  copy.mockRejectedValueOnce(
    coded("ALREADY_IN_WORLD", { filenames: ["old-a.zip", "old-b.zip"] })
  )
  copy.mockResolvedValueOnce("My World")
  const root = mount([row("terralith")], [world("My World")])
  await flush()
  within(rowOf(root, "terralith"), "addon-add-to-world-item")[0].click()
  await flush()
  expect(io.openModal).toHaveBeenCalledOnce()
  const [modal, question] = io.openModal.mock.calls[0]
  expect(modal).toEqual({ name: "confirmChoice" })
  expect(question).toMatchObject({
    title: "content:_trn_worlds.replace_title(world=My World)",
    message: "content:_trn_worlds.replace_files(files=old-a.zip, old-b.zip)",
    confirmLabel: "content:_trn_worlds.replace"
  })
  expect(io.toastError).not.toHaveBeenCalled()
  question.onConfirm()
  await flush()
  expect(copy).toHaveBeenCalledTimes(2)
  expect(copy.mock.calls[1][0]).toEqual({
    instance_id: 5,
    mod_id: "terralith",
    world: { name: "My World", identity: "id-My World" },
    replace: true
  })
  expect(io.openModal).toHaveBeenCalledOnce()
  expect(io.toastSuccess).toHaveBeenCalledOnce()
})

it("sends nothing more when the replace question is cancelled", async () => {
  const copy = mutation("instance.copyDatapack")
  copy.mockRejectedValueOnce(
    coded("ALREADY_IN_WORLD", { filenames: ["old.zip"] })
  )
  const root = mount([row("terralith")], [world("My World")])
  await flush()
  within(rowOf(root, "terralith"), "addon-add-to-world-item")[0].click()
  await flush()
  const question = io.openModal.mock.calls[0][1]

  // The question itself: Cancel closes it, only the primary button acts.
  const dialog = document.createElement("div")
  document.body.append(dialog)
  const onConfirm = vi.fn()
  const end = render(
    () => <ConfirmChoice title="" data={{ ...question, onConfirm }} />,
    dialog
  )
  expect(dialog.querySelector("h1")!.textContent).toBe(question.title)
  dialog
    .querySelector<HTMLButtonElement>('[data-testid="confirm-choice-cancel"]')!
    .click()
  expect(io.closeModal).toHaveBeenCalledOnce()
  expect(onConfirm).not.toHaveBeenCalled()
  end()
  await flush()
  expect(copy).toHaveBeenCalledOnce()
})

it("leaves any other refusal of the copy to the core's message", async () => {
  mutation("instance.copyDatapack").mockRejectedValueOnce(
    new Error("The world is gone")
  )
  const root = mount([row("terralith")], [world("My World")])
  await flush()
  within(rowOf(root, "terralith"), "addon-add-to-world-item")[0].click()
  await flush()
  expect(io.openModal).not.toHaveBeenCalled()
  expect(io.toastSuccess).not.toHaveBeenCalled()
  expect(mutation("instance.copyDatapack")).toHaveBeenCalledOnce()
})

it("offers Update everywhere when several copies of the pack have an update", async () => {
  mutation("instance.updateDatapackEverywhere").mockResolvedValue([
    { place: null, ok: true, error: null },
    { place: "Beta", ok: false, error: "Disk full" }
  ])
  const root = mount(
    [row("terralith", { modrinth: modrinth("TL"), has_update: true })],
    [
      world("Beta", [
        pack("b1", { modrinth: modrinth("TL"), has_update: true })
      ])
    ]
  )
  await flush()
  const element = rowOf(root, "terralith")
  expect(within(element, "mod-row-update-everywhere")[0].textContent).toBe(
    "content:_trn_worlds.update_everywhere2"
  )
  within(element, "mod-row-update-everywhere")[0].click()
  await flush()
  expect(
    mutation("instance.updateDatapackEverywhere")
  ).toHaveBeenCalledExactlyOnceWith({
    instance_id: 5,
    origin: { Instance: "terralith" }
  })
  expect(io.toastError).toHaveBeenCalledExactlyOnceWith(
    "content:_trn_worlds.update_failed_in(places=Beta)",
    { description: "Disk full" }
  )
})

it("updates here from the menu through the instance's own update", async () => {
  mutation("instance.updateMod").mockResolvedValue(73)
  const root = mount(
    [row("terralith", { modrinth: modrinth("TL"), has_update: true })],
    [
      world("Beta", [
        pack("b1", { modrinth: modrinth("TL"), has_update: true })
      ])
    ]
  )
  await flush()
  within(rowOf(root, "terralith"), "mod-row-update-here")[0].click()
  await flush()
  expect(mutation("instance.updateMod")).toHaveBeenCalledExactlyOnceWith({
    instance_id: 5,
    mod_id: "terralith"
  })
  expect(mutation("instance.updateDatapackEverywhere")).not.toHaveBeenCalled()
})

it("keeps the plain update click for a data pack with one copy", async () => {
  mutation("instance.updateMod").mockResolvedValue(73)
  const root = mount(
    [row("terralith", { modrinth: modrinth("TL"), has_update: true })],
    [world("Beta", [pack("b1", { modrinth: modrinth("TL") })])]
  )
  await flush()
  const element = rowOf(root, "terralith")
  expect(within(element, "mod-row-update-everywhere")).toHaveLength(0)
  within(element, "mod-row-update")[0].click()
  await flush()
  expect(mutation("instance.updateMod")).toHaveBeenCalledExactlyOnceWith({
    instance_id: 5,
    mod_id: "terralith"
  })
})

it("refuses the new actions while the game runs, saying why", async () => {
  const root = mount(
    [row("terralith", { modrinth: modrinth("TL"), has_update: true })],
    [
      world("Beta", [
        pack("b1", { modrinth: modrinth("TL"), has_update: true })
      ])
    ],
    { modloaders: [{}], modpack: null, state: { state: "running" } }
  )
  await flush()
  const element = rowOf(root, "terralith")
  expect(within(element, "addon-add-to-world-item")).toHaveLength(0)
  expect(within(element, "addon-add-to-world")[0].dataset.disabled).toBe("true")
  expect(within(element, "mod-row-update-everywhere")).toHaveLength(0)
  within(element, "mod-row-update")[0].click()
  await flush()
  expect(element.textContent).toContain("content:_trn_worlds.busy")
  expect(mutation("instance.updateMod")).not.toHaveBeenCalled()
  expect(mutation("instance.copyDatapack")).not.toHaveBeenCalled()
})

it("gives a server's data packs no Add to a world and no update menu", async () => {
  const [state] = createStore({
    rows: [],
    worlds: [world("Beta")],
    details: undefined,
    serverRows: [
      {
        id: "srv-pack",
        filename: "srv-pack.zip",
        displayName: "Server pack",
        addonType: "datapacks",
        enabled: true,
        hasImage: false,
        modrinthProjectId: "TL",
        curseforgeProjectId: null
      }
    ]
  })
  io.state = state
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <ServerAddons />, root)
  await flush()
  const element = rowOf(root, "srv-pack")
  expect(element).not.toBeNull()
  expect(within(element, "addon-add-to-world")).toHaveLength(0)
  expect(within(element, "mod-row-update-everywhere")).toHaveLength(0)
  expect(mutation("instance.copyDatapack")).not.toHaveBeenCalled()
})
