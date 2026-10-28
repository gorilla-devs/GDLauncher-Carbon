import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createRoot, createSignal } from "solid-js"
import { createStore } from "solid-js/store"
import { render } from "solid-js/web"
import type { ShaderRecommendation } from "@gd/core_module/bindings"

const c = vi.hoisted(() => ({
  query: vi.fn(),
  open: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  mutations: new Map<string, ReturnType<typeof vi.fn>[]>(),
  download: undefined as undefined | (() => Promise<void>),
  outcome: (() => undefined) as () => any
}))
vi.mock("@/utils/rspcClient", async () => {
  const { createSignal } = await import("solid-js")
  return {
    rspc: {
      useContext: () => ({ client: { query: c.query } }),
      createQuery: () => ({
        get data() {
          return c.outcome()
        }
      }),
      createMutation: (options: () => { mutationKey: string }) => {
        const key = options().mutationKey
        const [accepted, setAccepted] = createSignal<number>()
        const mutateAsync = vi.fn(async () => {
          setAccepted(61)
          return 61
        })
        c.mutations.set(key, [...(c.mutations.get(key) ?? []), mutateAsync])
        return {
          mutateAsync,
          get isSuccess() {
            return accepted() !== undefined
          },
          get data() {
            return accepted()
          },
          isPending: false
        }
      }
    }
  }
})
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: c.open })
}))
vi.mock("@gd/ui", () => ({ toast: { error: c.error, success: c.success } }))
vi.mock("@gd/i18n", () => {
  const t = (key: string, vars?: any) =>
    vars?.title ? `${key}:${vars.title}` : key
  return { useTransContext: () => [t], i18n: { t } }
})
vi.mock("./hooks/useInstanceSearch", () => ({
  useInstanceSearch: () => ({
    searchQuery: () => "",
    setSearchQuery: vi.fn(),
    hoveredInstanceId: () => null,
    setHoveredInstanceId: vi.fn(),
    filteredInstances: () => [],
    shouldVirtualize: () => false
  })
}))
vi.mock("./hooks/useTaskProgress", async () => {
  const { createSignal } = await import("solid-js")
  return {
    useTaskProgress: () => {
      const [loading, setLoading] = createSignal(false)
      const [progress, setProgress] = createSignal<number | null>(null)
      return { loading, setLoading, progress, setProgress }
    }
  }
})
vi.mock("./components/InstanceDropdown", () => ({
  InstanceDropdown: () => null
}))
vi.mock("./components/InstallButton", () => ({
  InstallButton: (p: any) => {
    c.download = p.onDownload
    return <button onClick={p.onDownload}>Install</button>
  }
}))
import { useModInstallation } from "./hooks/useModInstallation"
import ModDownloadButton from "./index"

const recommendation: ShaderRecommendation = {
  kind: "RecommendLoader",
  recommended: "Iris",
  modloader_type: "Fabric",
  mc_version: "1.21.1",
  loader_modrinth_id: "iris"
}
const present: ShaderRecommendation = { kind: "LoaderPresent", loader: "Iris" }
function pending<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
const addon = (id = "shader-A", type = "shader") => ({
  id,
  type,
  platform: "modrinth",
  title: id
})
const owned = (id = "owned-A", project = "shader-A") => ({
  id,
  addon_type: "shaders",
  modrinth: { project_id: project, version_id: "old-version" }
})
let disposals: (() => void)[] = []
let setOutcome: (value: any) => void
beforeEach(() => {
  vi.clearAllMocks()
  c.mutations.clear()
  c.download = undefined
  ;[c.outcome, setOutcome] = createSignal<any>()
})
afterEach(() => {
  disposals.forEach((d) => d())
  disposals = []
  document.body.replaceChildren()
})
function hookOwner(fileId: string | undefined) {
  const props = { addon: addon(), fileId }
  const hook = createRoot((dispose) => {
    disposals.push(dispose)
    return useModInstallation(props as any)
  })
  return { props, hook }
}
function buttonOwner(
  fileId: string | undefined,
  type = "shader",
  server = false
) {
  const [props, update] = createStore<any>({
    addon: addon("shader-A", type),
    fileId,
    selectedInstanceId: 7,
    selectedServerId: server ? 4 : undefined,
    selectedInstanceMods: [owned()]
  })
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(() => <ModDownloadButton {...props} />, root)
  disposals.push(dispose)
  return { update, dispose, download: c.download! }
}
function mutation(key: string, index: number) {
  return c.mutations.get(key)![index]
}
function expected(
  fileId: string | undefined,
  replacement: string | null = null
) {
  return fileId
    ? {
        instance_id: 7,
        install_deps: true,
        replaces_mod: replacement,
        mod_source: { Modrinth: { project_id: "shader-A", version_id: fileId } }
      }
    : { instance_id: 7, mod_source: { Modrinth: "shader-A" } }
}

for (const fileId of ["version-A", undefined]) {
  for (const response of [recommendation, present]) {
    it(`dropdown retains clicked ${fileId ?? "latest"} through ${response.kind}`, async () => {
      const read = pending<ShaderRecommendation>()
      c.query.mockReturnValue(read.promise)
      const o = hookOwner(fileId)
      const attempt = o.hook.handleInstanceSelection(7)
      o.props.addon = { ...addon("19"), platform: "curseforge" } as any
      o.props.fileId = "42"
      read.resolve(response)
      await attempt
      expect(c.query).toHaveBeenCalledWith([
        "instance.checkShaderRequirements",
        7
      ])
      if (response.kind === "RecommendLoader") {
        expect(c.open).toHaveBeenCalledWith(
          { name: "shaderLoaderSetup" },
          expect.objectContaining({
            instanceId: 7,
            installLatest: !fileId,
            replacesMod: null,
            modSource: fileId
              ? { Modrinth: { project_id: "shader-A", version_id: fileId } }
              : undefined,
            latestModSource: fileId ? undefined : { Modrinth: "shader-A" }
          })
        )
      } else {
        expect(
          mutation(
            fileId ? "instance.installMod" : "instance.installLatestMod",
            0
          )
        ).toHaveBeenCalledWith(expected(fileId))
      }
    })
    it(`direct button retains clicked target/replacement/${fileId ?? "latest"} through ${response.kind}`, async () => {
      const read = pending<ShaderRecommendation>()
      c.query.mockReturnValue(read.promise)
      // Latest is unowned so its ordinary already-installed guard permits click.
      const o = buttonOwner(fileId)
      if (!fileId) o.update("selectedInstanceMods", [])
      const attempt = o.download()
      o.update({
        addon: { ...addon("19"), platform: "curseforge" },
        fileId: 42,
        selectedInstanceId: 9,
        selectedInstanceMods: [owned("owned-B", "shader-B")]
      })
      read.resolve(response)
      await attempt
      expect(c.query).toHaveBeenCalledWith([
        "instance.checkShaderRequirements",
        7
      ])
      if (response.kind === "RecommendLoader") {
        expect(c.open).toHaveBeenCalledWith(
          { name: "shaderLoaderSetup" },
          expect.objectContaining({
            instanceId: 7,
            installLatest: !fileId,
            replacesMod: fileId ? "owned-A" : null,
            modSource: fileId
              ? { Modrinth: { project_id: "shader-A", version_id: fileId } }
              : undefined,
            latestModSource: fileId ? undefined : { Modrinth: "shader-A" }
          })
        )
      } else {
        expect(
          mutation(
            fileId ? "instance.installMod" : "instance.installLatestMod",
            1
          )
        ).toHaveBeenCalledWith(expected(fileId, fileId ? "owned-A" : null))
      }
    })
  }
}
it("unchanged shader click uses the supported recommendation", async () => {
  c.query.mockResolvedValue(recommendation)
  const o = hookOwner("version-A")
  await o.hook.handleInstanceSelection(7)
  expect(c.open).toHaveBeenCalledWith(
    { name: "shaderLoaderSetup" },
    expect.objectContaining({ recommendation, instanceId: 7 })
  )
  expect(mutation("instance.installMod", 0)).not.toHaveBeenCalled()
})
it("non-shader direct click preserves exact artifact without requirements", async () => {
  const o = buttonOwner("version-A", "mod")
  await o.download()
  expect(c.query).not.toHaveBeenCalled()
  expect(mutation("instance.installMod", 1)).toHaveBeenCalledWith(
    expected("version-A")
  )
})
it("ordinary server click has no shader preflight", async () => {
  const o = buttonOwner("version-A", "shader", true)
  await o.download()
  expect(c.query).not.toHaveBeenCalled()
  expect(mutation("server.installServerMod", 0)).toHaveBeenCalledWith({
    serverId: 4,
    modSource: { Modrinth: { project_id: "shader-A", version_id: "version-A" } }
  })
})
it("ordinary failed preflight preserves captured direct fallback", async () => {
  c.query.mockRejectedValue(new Error("controlled requirements failure"))
  const o = buttonOwner("version-A")
  await o.download()
  expect(c.error).toHaveBeenCalledWith(
    "notifications:_trn_shader_preflight_failed"
  )
  expect(mutation("instance.installMod", 1)).toHaveBeenCalledWith(
    expected("version-A", "owned-A")
  )
})
it("world-copy intent and completion survive later selection", async () => {
  const o = buttonOwner("version-A", "world")
  o.update("selectedInstanceMods", [{ ...owned(), addon_type: "worlds" }])
  const attempt = o.download()
  o.update({
    addon: addon("shader-B", "datapack"),
    fileId: "version-B",
    selectedInstanceId: 9,
    selectedInstanceMods: []
  })
  await attempt
  expect(mutation("instance.installMod", 1)).toHaveBeenCalledWith(
    expected("version-A", "owned-A")
  )
  setOutcome({ type: "Succeeded" })
  await Promise.resolve()
  expect(c.success).toHaveBeenCalledWith(
    "content:_trn_world_version_copy_complete",
    expect.anything()
  )
})
it("retired wizard result cannot fall through to direct install", async () => {
  const read = pending<ShaderRecommendation>()
  c.query.mockReturnValue(read.promise)
  const o = buttonOwner("version-A")
  const attempt = o.download()
  o.dispose()
  read.resolve(recommendation)
  await attempt
  expect(c.open).not.toHaveBeenCalled()
  expect(mutation("instance.installMod", 1)).not.toHaveBeenCalled()
})
it("disposed loader-present click may continue with its captured background artifact", async () => {
  const read = pending<ShaderRecommendation>()
  c.query.mockReturnValue(read.promise)
  const o = buttonOwner("version-A")
  const attempt = o.download()
  o.dispose()
  o.update({
    addon: addon("shader-B"),
    fileId: "version-B",
    selectedInstanceId: 9
  })
  read.resolve(present)
  await attempt
  expect(mutation("instance.installMod", 1)).toHaveBeenCalledWith(
    expected("version-A", "owned-A")
  )
  expect(c.open).not.toHaveBeenCalled()
})
it.each(["Succeeded", "Failed"])(
  "accepted task %s feedback belongs to clicked title",
  async (kind) => {
    c.query.mockResolvedValue(present)
    const o = buttonOwner("version-A")
    await o.download()
    o.update({ addon: addon("shader-B", "datapack"), selectedInstanceId: 9 })
    setOutcome(
      kind === "Succeeded"
        ? { type: "Succeeded" }
        : {
            type: "Failed",
            value: {
              progress: {
                type: "Failed",
                value: { cause: [{ display: "synthetic failure" }] }
              }
            }
          }
    )
    await Promise.resolve()
    if (kind === "Succeeded")
      expect(c.success).toHaveBeenCalledWith(
        "notifications:_trn_addon_installed:shader-A",
        expect.anything()
      )
    else
      expect(c.error).toHaveBeenCalledWith(
        "notifications:_trn_addon_install_failed:shader-A",
        expect.anything()
      )
  }
)
