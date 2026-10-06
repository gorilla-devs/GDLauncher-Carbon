import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import ModpackDownloadButton from "./ModpackDownloadButton"
import ServerPackDownloadButton from "./ServerPackDownloadButton"
const control = vi.hoisted(() => ({
  query: vi.fn(),
  creates: [] as any[],
  mutations: {} as Record<string, any>,
  errors: [] as any[]
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { invalidateQueries: vi.fn() },
  rspc: {
    useContext: () => ({ client: { query: control.query } }),
    createMutation: (options: any) => {
      const opts = options()
      control.mutations[opts.mutationKey[0]] = opts
      return {
        mutate: (input: any) =>
          control.creates.push({ key: opts.mutationKey[0], input })
      }
    }
  }
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => (
    <button disabled={p.disabled} onClick={p.onClick}>
      {p.children}
    </button>
  ),
  Spinner: () => <span>loading</span>,
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div>{p.children}</div>,
  toast: {
    success: vi.fn(),
    error: (title: any, options?: { description?: string }) =>
      control.errors.push(
        options?.description ? `${title} | ${options.description}` : title
      )
  }
}))
let dispose: (() => void) | undefined
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  control.query.mockReset()
  control.creates = []
  control.errors = []
  control.mutations = {}
})
function fixture(
  fileId?: string,
  mainFileId: string | null = "historical",
  platform = "modrinth"
) {
  const addon: any = {
    platform,
    id: platform === "modrinth" ? "owner" : "123",
    title: "pack",
    mainFileId,
    serverPackFileId: platform === "modrinth" ? "mrpack-server" : "789",
    imageUrl: null
  }
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <>
        <ModpackDownloadButton addon={addon} fileId={fileId} />
        <ServerPackDownloadButton addon={addon} fileId={fileId} />
      </>
    ),
    root
  )
  return [...root.querySelectorAll("button")]
}
it("both actual pack buttons resolve current default instead of historical project entry", async () => {
  control.query.mockResolvedValue([{ id: "current" }])
  const buttons = fixture()
  buttons.forEach((b) => b.click())
  await flush()
  expect(
    control.creates.map((c) =>
      c.key === "instance.createInstance"
        ? c.input.version.Modpack.value.version_id
        : c.input.modpackSource.modrinth.version_id
    )
  ).toEqual(["current", "current"])
  expect(control.query).toHaveBeenCalledTimes(2)
  expect(buttons.every((b) => b.disabled)).toBe(true)
})
it("explicit version works for both with no mainFileId and no query", async () => {
  const buttons = fixture("chosen", null)
  buttons.forEach((b) => b.click())
  await flush()
  expect(control.creates).toHaveLength(2)
  expect(control.query).not.toHaveBeenCalled()
  expect(control.creates[0].input.version.Modpack.value.version_id).toBe(
    "chosen"
  )
  expect(control.creates[1].input.modpackSource.modrinth.version_id).toBe(
    "chosen"
  )
})
it("both Modrinth defaults remain installable without mainFileId", async () => {
  control.query.mockResolvedValue([{ id: "current" }])
  fixture(undefined, null).forEach((b) => b.click())
  await flush()
  expect(control.creates[0].input.version.Modpack.value.version_id).toBe(
    "current"
  )
  expect(control.creates[1].input.modpackSource.modrinth.version_id).toBe(
    "current"
  )
})
it.each([undefined, "456"])(
  "CurseForge file selection %s uses main or verified release-specific server companion",
  async (selected) => {
    if (selected)
      control.query.mockResolvedValue({
        data: { id: 456, modId: 123, isAvailable: true, serverPackFileId: 790 }
      })
    fixture(selected, "321", "curseforge").forEach((b) => b.click())
    await flush()
    if (selected) {
      expect(control.query).toHaveBeenCalledWith([
        "modplatforms.curseforge.getModFile",
        { modId: 123, fileId: 456 }
      ])
    } else expect(control.query).not.toHaveBeenCalled()
    const expected = Number(selected ?? "321")
    expect(
      control.creates.find((c) => c.key === "instance.createInstance").input
        .version.Modpack.value.file_id
    ).toBe(expected)
    expect(
      control.creates.find((c) => c.key === "server.createServerFromModpack")
        .input.modpackSource.curseforge
    ).toEqual({
      project_id: 123,
      file_id: expected,
      server_pack_file_id: selected ? 790 : 789
    })
  }
)

it.each(["empty", "rejected"])(
  "resolution %s clears loading, reports error and permits retry without creation",
  async (kind) => {
    if (kind === "empty") control.query.mockResolvedValue([])
    else
      control.query.mockRejectedValue(
        new Error("synthetic versions unavailable")
      )
    const buttons = fixture()
    buttons.forEach((b) => b.click())
    await flush()
    expect(control.creates).toHaveLength(0)
    expect(control.errors).toHaveLength(2)
    expect(buttons.every((b) => !b.disabled)).toBe(true)
    control.query.mockResolvedValue([{ id: "recovered" }])
    buttons.forEach((b) => b.click())
    await flush()
    expect(control.creates).toHaveLength(2)
    expect(buttons.every((b) => b.disabled)).toBe(true)
    control.mutations["instance.createInstance"].onError()
    control.mutations["server.createServerFromModpack"].onError(
      new Error("synthetic mutation failure")
    )
    expect(buttons.every((b) => !b.disabled)).toBe(true)
  }
)
// Refusals are written for players in their language. The backend's own
// lookup error is shown as-is, under the same translated title.
it.each([
  [
    "no Modrinth versions",
    () => control.query.mockResolvedValue([]),
    "modrinth",
    undefined,
    [
      "notifications:_trn_pack_version_failed | notifications:_trn_pack_no_versions",
      "notifications:_trn_pack_version_failed | notifications:_trn_pack_no_versions"
    ]
  ],
  [
    "a CurseForge release without a server pack",
    () =>
      control.query.mockResolvedValue({
        data: { id: 456, modId: 123, isAvailable: true, serverPackFileId: null }
      }),
    "curseforge",
    "456",
    [
      "notifications:_trn_pack_version_failed | notifications:_trn_server_pack_unavailable"
    ]
  ],
  [
    "an unavailable CurseForge release",
    () =>
      control.query.mockResolvedValue({
        data: { id: 456, modId: 123, isAvailable: false, serverPackFileId: 790 }
      }),
    "curseforge",
    "456",
    [
      "notifications:_trn_pack_version_failed | notifications:_trn_curseforge_file_unavailable"
    ]
  ],
  [
    "a failed version lookup",
    () => control.query.mockRejectedValue(new Error("Modrinth returned 429")),
    "modrinth",
    undefined,
    [
      "notifications:_trn_pack_version_failed | Modrinth returned 429",
      "notifications:_trn_pack_version_failed | Modrinth returned 429"
    ]
  ]
] as const)(
  "%s shows a translated refusal",
  async (_, arrange, platform, fileId, errors) => {
    arrange()
    fixture(fileId, platform === "curseforge" ? "321" : null, platform).forEach(
      (b) => b.click()
    )
    await flush()
    expect(control.errors).toEqual(errors)
  }
)
