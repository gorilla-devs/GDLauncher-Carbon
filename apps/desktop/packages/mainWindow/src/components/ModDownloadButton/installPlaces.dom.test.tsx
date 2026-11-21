import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query"

// The real download button and the real @gd/ui menu (Kobalte), with an
// instance already chosen as the target. Transport and toasts are stubbed.
const c = vi.hoisted(() => ({
  query: vi.fn(),
  mutate: vi.fn(),
  success: vi.fn(),
  error: vi.fn()
}))
vi.mock("@/utils/rspcClient", async () => {
  const { createQuery, createMutation, useQueryClient } =
    await import("@tanstack/solid-query")
  return {
    rspc: {
      useContext: () => ({
        client: { query: c.query },
        queryClient: useQueryClient()
      }),
      createQuery: (options: any) =>
        createQuery(() => ({
          ...options(),
          queryFn: () => c.query(options().queryKey)
        })),
      createMutation: (options: any) =>
        createMutation(() => {
          const key = options().mutationKey
          return {
            mutationKey: [key],
            mutationFn: (input: any) => c.mutate(key, input)
          }
        })
    }
  }
})
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@gd/ui", async (original) => ({
  ...(await original<typeof import("@gd/ui")>()),
  toast: { error: c.error, success: c.success }
}))
vi.mock("@gd/i18n", () => {
  const t = (key: string, vars?: any) =>
    vars?.world
      ? `${key}:${vars.name}@${vars.world}`
      : vars?.title
        ? `${key}:${vars.title}`
        : vars?.version
          ? `${key}:${vars.version}`
          : key
  return {
    Trans: (p: any) => <span>{p.key}</span>,
    useTransContext: () => [t],
    i18n: { t }
  }
})
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    instances: {
      data: [
        {
          id: 7,
          name: "Fabric",
          status: {
            status: "valid",
            value: { mc_version: "1.21.1", modloader: "fabric" }
          }
        },
        {
          id: 8,
          name: "Vanilla",
          status: {
            status: "valid",
            value: { mc_version: "1.21.1", modloader: null }
          }
        }
      ]
    }
  })
}))
vi.mock("@/utils/instances", () => ({ getInstanceImageUrl: () => "icon" }))
vi.mock("@/components/VirtuaWrapper", () => ({ VList: () => null }))
import ModDownloadButton from "./index"
import { SearchInputContext } from "@/components/SearchInputContext"

// The four shapes a search row comes in.
const curseforgePack = {
  id: "12",
  title: "Pack",
  platform: "curseforge",
  type: "datapack",
  loaders: []
}
const modrinthPack = {
  id: "P",
  title: "Pack",
  platform: "modrinth",
  type: "mod",
  loaders: ["datapack"]
}
const modrinthHybrid = {
  ...modrinthPack,
  loaders: ["datapack", "fabric", "forge", "neoforge", "quilt"]
}
const plainMod = { ...modrinthPack, id: "M", loaders: ["fabric"] }

const world = (
  name: string,
  datapacks: any[] = [],
  identity: string | null = `id-${name}`
) => ({ name, addon_id: `addon-${name}`, identity, datapacks })
const curseforgeCopy = (id: string) => ({
  id,
  addon_type: "datapacks",
  curseforge: { project_id: 12, file_id: 3 },
  modrinth: null
})
const running = {
  type: "Running",
  value: { progress: { type: "Known", value: 0.2 } }
}

const cleanups: (() => void)[] = []
beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(window, "scrollTo").mockImplementation(() => {})
})
afterEach(() => {
  cleanups.splice(0).forEach((fn) => fn())
  document.body.replaceChildren()
  vi.restoreAllMocks()
})
const flush = async () => {
  for (let i = 0; i < 8; i++) {
    await Promise.resolve()
    await new Promise((r) => setTimeout(r, 0))
  }
}
function serve(worlds: any[] | (() => never)) {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds"
      ? typeof worlds === "function"
        ? worlds()
        : worlds
      : running
  )
  c.mutate.mockResolvedValue(61)
}
async function mount(
  addon: object,
  props: Record<string, unknown> = {},
  searched?: string
) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { staleTime: Infinity, retry: false },
      mutations: { retry: false }
    }
  })
  const search = searched
    ? ({ searchQuery: () => ({ projectType: searched }) } as any)
    : undefined
  // Instance 7 unless the test names another target, or none.
  const target = {
    selectedInstanceId: 7,
    selectedInstanceMods: [],
    testId: "download",
    ...props
  }
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <SearchInputContext.Provider value={search}>
          <ModDownloadButton addon={addon as any} {...target} />
        </SearchInputContext.Provider>
      </QueryClientProvider>
    ),
    root
  )
  cleanups.push(() => {
    dispose()
    client.clear()
  })
  await flush()
  return client
}
const button = () =>
  document.querySelector<HTMLButtonElement>('[data-testid="download"]')!
async function download() {
  button().dispatchEvent(
    new MouseEvent("pointerdown", { button: 0, bubbles: true })
  )
  button().click()
  await flush()
}
// Kobalte selects a menu item on pointer up.
async function pick(text: string) {
  menuItems()
    .find((item) => item.textContent?.includes(text))!
    .dispatchEvent(new MouseEvent("pointerup", { button: 0, bubbles: true }))
  await flush()
}
// The trigger says whether the menu is open; jsdom keeps closed content mounted.
const isOpen = () =>
  document.querySelector("[aria-haspopup]")?.getAttribute("aria-expanded") ===
  "true"
const menuItems = () =>
  isOpen()
    ? [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
    : []
const items = () => menuItems().map((item) => item.textContent?.trim())
const requests = () => c.mutate.mock.calls.map(([key]) => key)
const worldsRead = () =>
  c.query.mock.calls.some(([key]) => key[0] === "instance.getWorlds")

it.each([
  ["a CurseForge data pack", curseforgePack, "datapack"],
  ["a Modrinth data pack", modrinthPack, "datapack"],
  ["a Modrinth data pack in a mod search", modrinthPack, "mod"],
  ["a Modrinth data pack outside a search", modrinthPack, undefined],
  ["a Modrinth data pack that also ships as a mod", modrinthHybrid, "datapack"]
])(
  "Download of %s asks where: the instance's worlds, then the instance",
  async (_, addon, searched) => {
    serve([world("My World"), world("Broken", [], null), world("Skyblock")])
    await mount(addon, {}, searched)
    expect(button().textContent).toContain("instances:_trn_download")
    await download()
    expect(requests()).toEqual([])
    expect(items()).toEqual([
      "My World",
      "Skyblock",
      "content:_trn_worlds.download_to_instance"
    ])
    expect(document.body.textContent).toContain("content:_trn_tabs.worlds")
    // The left panel already names the instance: no header with it.
    expect(document.querySelector(".i-hugeicons\\:arrow-left-01")).toBeNull()
  }
)
it("a world picked there gets the data pack, and its completion names the world", async () => {
  serve([world("My World"), world("Skyblock")])
  const client = await mount(modrinthHybrid, {}, "datapack")
  await download()
  await pick("Skyblock")
  expect(c.mutate.mock.calls).toEqual([
    [
      "instance.installWorldDatapack",
      {
        instance_id: 7,
        world: "Skyblock",
        identity: "id-Skyblock",
        source: { Modrinth: { project_id: "P", version_id: null } },
        replaces: null
      }
    ]
  ])
  expect(isOpen()).toBe(false)
  client.setQueryData(["vtask.getTaskOutcome", 61], { type: "Succeeded" })
  await flush()
  expect(c.success).toHaveBeenCalledWith(
    "content:_trn_worlds.added_to_world:Pack@Skyblock",
    { duration: 2000 }
  )
})
it("Download to instance picked there installs into the instance", async () => {
  serve([world("My World")])
  await mount(curseforgePack, {}, "datapack")
  await download()
  await pick("content:_trn_worlds.download_to_instance")
  expect(c.mutate.mock.calls).toEqual([
    [
      "instance.installLatestMod",
      { instance_id: 7, mod_source: { Curseforge: 12 } }
    ]
  ])
  expect(isOpen()).toBe(false)
})
it.each([
  ["no world", []],
  ["only unreadable worlds", [world("Broken", [], null)]],
  [
    "worlds that cannot be read",
    () => {
      throw new Error("unreadable")
    }
  ]
])(
  "an instance with %s gets the data pack at once, with no menu",
  async (_, worlds) => {
    serve(worlds as any)
    await mount(curseforgePack, {}, "datapack")
    await download()
    expect(isOpen()).toBe(false)
    expect(c.mutate.mock.calls).toEqual([
      [
        "instance.installLatestMod",
        { instance_id: 7, mod_source: { Curseforge: 12 } }
      ]
    ])
  }
)
it("shows Installed once every world and the instance hold the data pack", async () => {
  serve([world("My World", [curseforgeCopy("in-world")])])
  await mount(
    curseforgePack,
    { selectedInstanceMods: [curseforgeCopy("in-instance")] },
    "datapack"
  )
  expect(button().textContent).toContain("content:_trn_mod.downloaded")
  await download()
  expect(isOpen()).toBe(false)
  expect(requests()).toEqual([])
})
it("still offers a world that can take a data pack the instance holds, the instance ticked", async () => {
  serve([world("My World"), world("Holder", [curseforgeCopy("in-world")])])
  await mount(
    curseforgePack,
    { selectedInstanceMods: [curseforgeCopy("in-instance")] },
    "datapack"
  )
  expect(button().textContent).toContain("instances:_trn_download")
  await download()
  const disabled = menuItems()
    .filter((item) => item.hasAttribute("data-disabled"))
    .map((item) => item.textContent?.trim())
  expect(disabled).toEqual([
    "Holder",
    "content:_trn_worlds.download_to_instance"
  ])
  await pick("My World")
  expect(requests()).toEqual(["instance.installWorldDatapack"])
})
it.each([
  [
    "a Modrinth data pack that also ships as a mod, in a mod search",
    modrinthHybrid,
    "mod"
  ],
  ["that project outside a search", modrinthHybrid, undefined],
  ["a mod, even in a data pack search", plainMod, "datapack"],
  ["a shader", { ...plainMod, type: "shader", loaders: [] }, "shader"],
  [
    "a resource pack",
    { ...plainMod, type: "resourcePack", loaders: [] },
    "resourcePack"
  ],
  ["a world", { ...plainMod, type: "world", loaders: [] }, "world"]
])(
  "Download of %s installs in one click, with no menu",
  async (_, addon, searched) => {
    serve([world("My World")])
    c.query.mockImplementation(async ([key]: any) =>
      key === "instance.checkShaderRequirements"
        ? { kind: "LoaderPresent", loader: "Iris" }
        : key === "instance.getWorlds"
          ? [world("My World")]
          : running
    )
    await mount(addon, {}, searched)
    await download()
    expect(isOpen()).toBe(false)
    expect(c.mutate.mock.calls).toEqual([
      [
        "instance.installLatestMod",
        { instance_id: 7, mod_source: { Modrinth: addon.id } }
      ]
    ])
    expect(worldsRead()).toBe(false)
  }
)
it.each([
  ["a CurseForge data pack", curseforgePack, { Curseforge: 12 }],
  ["a Modrinth data pack", modrinthPack, { Modrinth: "P" }]
])(
  "Download of %s for a server installs in one click, with no menu",
  async (_, addon, modSource) => {
    serve([world("My World")])
    await mount(
      addon,
      {
        selectedInstanceId: undefined,
        selectedServerId: 9,
        selectedServerAddons: []
      },
      "datapack"
    )
    await download()
    expect(isOpen()).toBe(false)
    expect(c.mutate.mock.calls).toEqual([
      ["server.installLatestServerMod", { serverId: 9, modSource }]
    ])
    expect(worldsRead()).toBe(false)
  }
)
it("a version picked in the Versions tab installs into the instance at once", async () => {
  serve([world("My World")])
  await mount(curseforgePack, { fileId: "34", pickedVersion: true }, "datapack")
  expect(button().textContent).toContain("instances:_trn_download_version")
  await download()
  expect(isOpen()).toBe(false)
  expect(c.mutate.mock.calls).toEqual([
    [
      "instance.installMod",
      {
        instance_id: 7,
        mod_source: { Curseforge: { project_id: 12, file_id: 34 } },
        install_deps: true,
        replaces_mod: null
      }
    ]
  ])
  expect(worldsRead()).toBe(false)
})
it("the menu opened before the worlds are read installs into the instance once they turn out to be none", async () => {
  let answer!: (worlds: any[]) => void
  c.query.mockImplementation(([key]: any) =>
    key === "instance.getWorlds"
      ? new Promise((resolve) => {
          answer = resolve
        })
      : Promise.resolve(running)
  )
  c.mutate.mockResolvedValue(61)
  await mount(curseforgePack, {}, "datapack")
  await download()
  expect(isOpen()).toBe(true)
  expect(requests()).toEqual([])
  answer([])
  await flush()
  expect(isOpen()).toBe(false)
  expect(requests()).toEqual(["instance.installLatestMod"])
})

// An install into the instance itself. Modrinth lists the data pack builds
// of a project for a game version; the newest release is the one installed.
const builds = [
  {
    id: "V-beta",
    version_type: "beta",
    date_published: "2026-02-01T00:00:00Z",
    files: [{ filename: "pack-beta.zip", primary: true }]
  },
  {
    id: "V-release",
    version_type: "release",
    date_published: "2026-01-01T00:00:00Z",
    files: [{ filename: "pack.zip", primary: true }]
  }
]
function serveBuilds(worlds: any[], versions: any[] | (() => never) = builds) {
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.getWorlds"
      ? worlds
      : key === "modplatforms.modrinth.getProjectVersions"
        ? typeof versions === "function"
          ? versions()
          : versions
        : key === "instance.getInstanceMods"
          ? []
          : running
  )
  c.mutate.mockResolvedValue(61)
}
const buildReads = () =>
  c.query.mock.calls
    .map(([key]) => key)
    .filter((key) => key[0] === "modplatforms.modrinth.getProjectVersions")
const buildsAsked = [
  [
    "modplatforms.modrinth.getProjectVersions",
    { project_id: "P", game_versions: ["1.21.1"], loaders: ["datapack"] }
  ]
]
const buildInstall = (instance: number) => [
  "instance.installMod",
  {
    instance_id: instance,
    mod_source: { Modrinth: { project_id: "P", version_id: "V-release" } },
    install_deps: true,
    replaces_mod: null
  }
]
it.each([
  ["a Modrinth data pack on an instance with a mod loader", modrinthPack, 7],
  ["a Modrinth data pack on an instance without one", modrinthPack, 8],
  [
    "a Modrinth data pack that also ships as a mod on an instance without a mod loader",
    modrinthHybrid,
    8
  ]
])(
  "Download to instance of %s installs its newest data pack build as a data pack",
  async (_, addon, instance) => {
    serveBuilds([world("My World")])
    const client = await mount(
      addon,
      { selectedInstanceId: instance },
      "datapack"
    )
    await download()
    await pick("content:_trn_worlds.download_to_instance")
    expect(buildReads()).toEqual(buildsAsked)
    expect(c.mutate.mock.calls).toEqual([buildInstall(instance)])
    client.setQueryData(["vtask.getTaskOutcome", 61], { type: "Succeeded" })
    await flush()
    expect(c.success).toHaveBeenCalledWith(
      "notifications:_trn_datapack_downloaded:Pack",
      { duration: 2000 }
    )
  }
)
it("Download to instance of a Modrinth data pack that also ships as a mod installs its mod where a mod loader runs it", async () => {
  serveBuilds([world("My World")])
  const client = await mount(modrinthHybrid, {}, "datapack")
  await download()
  await pick("content:_trn_worlds.download_to_instance")
  expect(buildReads()).toEqual([])
  expect(c.mutate.mock.calls).toEqual([
    [
      "instance.installLatestMod",
      { instance_id: 7, mod_source: { Modrinth: "P" } }
    ]
  ])
  client.setQueryData(["vtask.getTaskOutcome", 61], { type: "Succeeded" })
  await flush()
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:Pack",
    { duration: 2000 }
  )
})
it("an instance without a mod loader and without a world gets a Modrinth data pack build at once", async () => {
  serveBuilds([])
  await mount(modrinthPack, { selectedInstanceId: 8 }, "datapack")
  await download()
  expect(isOpen()).toBe(false)
  expect(buildReads()).toEqual(buildsAsked)
  expect(c.mutate.mock.calls).toEqual([buildInstall(8)])
})
it.each([
  ["no version", []],
  [
    "only a jar",
    [{ ...builds[1], files: [{ filename: "mod.jar", primary: true }] }]
  ]
])(
  "says the project has no data pack for the instance's Minecraft when Modrinth lists %s",
  async (_, versions) => {
    serveBuilds([], versions)
    await mount(modrinthPack, { selectedInstanceId: 8 }, "datapack")
    await download()
    expect(requests()).toEqual([])
    expect(c.error.mock.calls).toEqual([
      ["content:_trn_worlds.no_datapack_for_version:1.21.1"]
    ])
    expect(button().textContent).toContain("instances:_trn_download")
  }
)
it("says once that the install failed when the builds cannot be listed", async () => {
  serveBuilds([], () => {
    throw new Error("Modrinth is unreachable")
  })
  await mount(modrinthPack, { selectedInstanceId: 8 }, "datapack")
  await download()
  expect(requests()).toEqual([])
  expect(c.error.mock.calls).toEqual([
    [
      "notifications:_trn_addon_install_failed:Pack",
      { description: "Modrinth is unreachable" }
    ]
  ])
  expect(button().textContent).toContain("instances:_trn_download")
})
it("a Modrinth data pack build the instance holds reads as downloaded, to add to a world", async () => {
  serveBuilds([])
  await mount(modrinthPack, {
    fileId: "V1",
    pickedVersion: true,
    selectedInstanceMods: [
      {
        id: "copy",
        addon_type: "datapacks",
        modrinth: { project_id: "P", version_id: "V1" },
        curseforge: null
      }
    ]
  })
  expect(button().textContent).toContain("content:_trn_datapack_downloaded")
})
it("ticks Download to instance for a Modrinth data pack build the instance holds", async () => {
  serveBuilds([world("My World")])
  await mount(
    modrinthPack,
    {
      selectedInstanceMods: [
        {
          id: "copy",
          addon_type: "datapacks",
          modrinth: { project_id: "P", version_id: "V1" },
          curseforge: null
        }
      ]
    },
    "datapack"
  )
  await download()
  expect(
    menuItems()
      .filter((item) => item.hasAttribute("data-disabled"))
      .map((item) => item.textContent?.trim())
  ).toEqual(["content:_trn_worlds.download_to_instance"])
})

// The instance picker, with no instance chosen yet.
async function openPicker() {
  const trigger = [
    ...document.querySelectorAll<HTMLElement>("[aria-haspopup]")
  ].find((item) => item.textContent?.includes("_trn_add_to_an_instance"))!
  trigger.dispatchEvent(
    new MouseEvent("pointerdown", { button: 0, bubbles: true })
  )
  trigger.click()
  await flush()
}
const pickerRow = (name: string) =>
  [...document.querySelectorAll<HTMLElement>('[role="menuitemradio"]')].find(
    (item) => item.textContent?.includes(name)
  )!
async function pickRow(name: string) {
  pickerRow(name).dispatchEvent(
    new MouseEvent("pointerup", { button: 0, bubbles: true })
  )
  await flush()
}
it.each([
  ["that is only a data pack", modrinthPack],
  ["that also ships as a mod", modrinthHybrid]
])(
  "the picker offers Download to instance for a Modrinth project %s on an instance without a mod loader, and installs its data pack build",
  async (_, addon) => {
    serveBuilds([world("My World")])
    await mount(addon, { selectedInstanceId: undefined }, "datapack")
    await openPicker()
    await pickRow("Vanilla")
    expect(items()).toEqual([
      "My World",
      "content:_trn_worlds.download_to_instance"
    ])
    await pick("content:_trn_worlds.download_to_instance")
    expect(buildReads()).toEqual(buildsAsked)
    expect(c.mutate.mock.calls).toEqual([buildInstall(8)])
    expect(pickerRow("Vanilla").textContent).toContain(
      "instances:_trn_status_downloading"
    )
  }
)
it("the picker installs a Modrinth data pack build at once into an instance without a mod loader and without a world", async () => {
  serveBuilds([])
  await mount(modrinthPack, { selectedInstanceId: undefined }, "datapack")
  await openPicker()
  await pickRow("Vanilla")
  expect(buildReads()).toEqual(buildsAsked)
  expect(c.mutate.mock.calls).toEqual([buildInstall(8)])
})
it("the picker installs the mod of a Modrinth data pack that also ships as a mod on an instance with a mod loader", async () => {
  serveBuilds([world("My World")])
  await mount(modrinthHybrid, { selectedInstanceId: undefined }, "datapack")
  await openPicker()
  await pickRow("Fabric")
  await pick("content:_trn_worlds.download_to_instance")
  expect(buildReads()).toEqual([])
  expect(c.mutate.mock.calls).toEqual([
    [
      "instance.installLatestMod",
      { instance_id: 7, mod_source: { Modrinth: "P" } }
    ]
  ])
  expect(pickerRow("Fabric").textContent).toContain(
    "instances:_trn_status_installing"
  )
})
it("the picker says the project has no data pack for the instance's Minecraft and frees the row", async () => {
  serveBuilds([], [])
  await mount(modrinthPack, { selectedInstanceId: undefined }, "datapack")
  await openPicker()
  await pickRow("Vanilla")
  expect(requests()).toEqual([])
  expect(c.error.mock.calls).toEqual([
    ["content:_trn_worlds.no_datapack_for_version:1.21.1"]
  ])
  expect(pickerRow("Vanilla").textContent).not.toContain("_trn_status_")
})

it("a world install that failed is told in the player's language and has the worlds read again", async () => {
  serve([world("My World")])
  const client = await mount(modrinthPack, {}, "datapack")
  await download()
  await pick("My World")
  expect(requests()).toEqual(["instance.installWorldDatapack"])
  const worldReads = () =>
    c.query.mock.calls.filter(([key]) => key[0] === "instance.getWorlds").length
  const before = worldReads()
  client.setQueryData(["vtask.getTaskOutcome", 61], {
    type: "Failed",
    value: {
      progress: {
        type: "Failed",
        value: {
          cause: [
            {
              display: "Selected world changed during copying",
              code: "WORLD_CHANGED_DURING_COPY"
            }
          ]
        }
      }
    }
  })
  await flush()
  expect(c.error.mock.calls).toEqual([
    [
      "notifications:_trn_addon_install_failed:Pack",
      { description: "content:_trn_worlds.refusal.world_changed" }
    ]
  ])
  expect(worldReads()).toBe(before + 1)
})
