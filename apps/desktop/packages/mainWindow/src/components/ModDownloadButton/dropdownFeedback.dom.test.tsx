import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal, Show, Switch, Match } from "solid-js"
import { render } from "solid-js/web"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import type { ShaderRecommendation } from "@gd/core_module/bindings"

const c = vi.hoisted(() => ({
  query: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  open: vi.fn(),
  mutations: new Map<string, ReturnType<typeof vi.fn>[]>(),
  select: undefined as any,
  download: undefined as any,
  dropdownMounts: 0,
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
      createMutation: (options: any) => {
        const key = options().mutationKey
        const [id, setId] = createSignal<number>()
        const mutateAsync = vi.fn(async () => {
          setId(61)
          return 61
        })
        c.mutations.set(key, [...(c.mutations.get(key) ?? []), mutateAsync])
        return {
          mutateAsync,
          get data() {
            return id()
          },
          get isSuccess() {
            return id() !== undefined
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
vi.mock("./components/InstanceDropdown", () => ({
  InstanceDropdown: (p: any) => {
    c.dropdownMounts++
    c.select = p.handleInstanceSelection
    return <div>dropdown</div>
  }
}))
vi.mock("./components/InstallButton", () => ({
  InstallButton: (p: any) => {
    c.download = p.onDownload
    return <div>button</div>
  }
}))
import ModDownloadButton from "./index"

const recommendation: ShaderRecommendation = {
  kind: "RecommendLoader",
  recommended: "Iris",
  modloader_type: "Fabric",
  mc_version: "1.21.1",
  loader_modrinth_id: "iris"
}
const project = (id: string, type = "shader") => ({
  id,
  title: `${type}-${id}`,
  platform: "modrinth",
  type
})
const disposals: (() => void)[] = []
let setOutcome: (value: any) => void
beforeEach(() => {
  vi.clearAllMocks()
  c.mutations.clear()
  c.dropdownMounts = 0
  ;[c.outcome, setOutcome] = createSignal<any>({ type: "Succeeded" })
})
afterEach(() => {
  disposals.splice(0).forEach((d) => d())
  document.body.replaceChildren()
})
function pending() {
  let resolve!: (value: ShaderRecommendation) => void
  const promise = new Promise<ShaderRecommendation>((yes) => {
    resolve = yes
  })
  return { promise, resolve }
}
async function tick() {
  for (let n = 0; n < 8; n++) await Promise.resolve()
}
function owner(direct = false, typeB = "shader") {
  const client = new QueryClient({
    defaultOptions: { queries: { staleTime: Infinity, retry: false } }
  })
  client.setQueryData(["project", "A"], project("A"))
  client.setQueryData(["project", "B"], project("B", typeB))
  const [key, setKey] = createSignal("A")
  let actualProject: any
  const root = document.createElement("div")
  document.body.append(root)
  const Harness = () => {
    const query = createQuery(() => ({
      queryKey: ["project", key()],
      queryFn: async (): Promise<ReturnType<typeof project>> => {
        throw new Error("cache-only fixture")
      }
    }))
    actualProject = query
    // Preserve the AddonViewPage non-keyed latest Show/boolean Match owner.
    return (
      <Show when={true}>
        <Switch>
          <Match when={query.data?.type && query.data?.type !== "modpack"}>
            <ModDownloadButton
              addon={query.data as any}
              selectedInstanceId={direct ? 7 : undefined}
            />
          </Match>
        </Switch>
      </Show>
    )
  }
  const dispose = render(
    () => (
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    ),
    root
  )
  const cleanup = () => {
    dispose()
    client.clear()
  }
  disposals.push(cleanup)
  return { setKey, dispose: cleanup, actualProject: () => actualProject.data }
}
function configure(read: ReturnType<typeof pending>) {
  c.query.mockImplementation(([key]: [string]) =>
    key === "instance.checkShaderRequirements"
      ? read.promise
      : Promise.resolve(c.outcome())
  )
}
function expectBInstall(index = 0) {
  expect(
    c.mutations.get("instance.installLatestMod")![index]
  ).toHaveBeenCalledWith({ instance_id: 7, mod_source: { Modrinth: "B" } })
}
async function clickBThenA(direct = false) {
  const read = pending()
  configure(read)
  const o = owner(direct)
  await tick()
  o.setKey("B")
  await tick()
  expect(o.actualProject().id).toBe("B")
  if (!direct) expect(c.dropdownMounts).toBe(1)
  const clicked = direct ? c.download() : c.select(7)
  o.setKey("A")
  await tick()
  expect(o.actualProject().id).toBe("A")
  if (!direct) expect(c.dropdownMounts).toBe(1)
  return { read, clicked, o }
}
it("real dropdown feedback stays A for click A then cached B during delayed preflight", async () => {
  const read = pending()
  configure(read)
  const o = owner()
  await tick()
  const clicked = c.select(7)
  o.setKey("B")
  await tick()
  expect(o.actualProject().id).toBe("B")
  expect(c.dropdownMounts).toBe(1)
  read.resolve({ kind: "LoaderPresent", loader: "Iris" })
  await clicked
  await tick()
  expect(c.mutations.get("instance.installLatestMod")![0]).toHaveBeenCalledWith(
    { instance_id: 7, mod_source: { Modrinth: "A" } }
  )
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:shader-A",
    expect.anything()
  )
})
it("real dropdown feedback belongs to B clicked after a cached project switch", async () => {
  const { read, clicked } = await clickBThenA()
  read.resolve({ kind: "LoaderPresent", loader: "Iris" })
  await clicked
  await tick()
  expectBInstall()
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:shader-B",
    expect.anything()
  )
})
it("real direct button feedback belongs to B clicked after a cached project switch", async () => {
  const { read, clicked } = await clickBThenA(true)
  read.resolve({ kind: "LoaderPresent", loader: "Iris" })
  await clicked
  await tick()
  expectBInstall(1)
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_addon_installed:shader-B",
    expect.anything()
  )
})
it.each(["Failed", "Unavailable", "Dismissed"])(
  "real dropdown %s outcome keeps clicked ownership and its outcome distinction",
  async (kind) => {
    setOutcome(
      kind === "Failed"
        ? {
            type: "Failed",
            value: {
              progress: {
                type: "Failed",
                value: { cause: [{ display: "controlled failure" }] }
              }
            }
          }
        : { type: kind }
    )
    const { read, clicked } = await clickBThenA()
    read.resolve({ kind: "LoaderPresent", loader: "Iris" })
    await clicked
    await tick()
    expectBInstall()
    expect(c.success).not.toHaveBeenCalled()
    if (kind === "Dismissed") expect(c.error).not.toHaveBeenCalled()
    else
      expect(c.error).toHaveBeenCalledWith(
        "notifications:_trn_addon_install_failed:shader-B",
        expect.anything()
      )
  }
)
it.each(["Succeeded", "Failed"])(
  "wizard onComplete %s task feedback belongs to clicked B after later cached A",
  async (kind) => {
    setOutcome(
      kind === "Succeeded"
        ? { type: "Succeeded" }
        : {
            type: "Failed",
            value: {
              progress: {
                type: "Failed",
                value: { cause: [{ display: "controlled wizard failure" }] }
              }
            }
          }
    )
    const { read, clicked } = await clickBThenA()
    read.resolve(recommendation)
    await clicked
    expect(c.open).toHaveBeenCalledWith(
      { name: "shaderLoaderSetup" },
      expect.objectContaining({
        instanceId: 7,
        latestModSource: { Modrinth: "B" }
      })
    )
    expect(
      c.mutations.get("instance.installLatestMod")![0]
    ).not.toHaveBeenCalled()
    c.open.mock.calls[0][1].onComplete(61)
    await tick()
    expect(c.query).toHaveBeenCalledWith(["vtask.getTaskOutcome", 61])
    if (kind === "Succeeded")
      expect(c.success).toHaveBeenCalledWith(
        "notifications:_trn_addon_installed:shader-B",
        expect.anything()
      )
    else
      expect(c.error).toHaveBeenCalledWith(
        "notifications:_trn_addon_install_failed:shader-B",
        expect.anything()
      )
  }
)
it("real dropdown completion carries clicked datapack type after later cached shader", async () => {
  c.query.mockImplementation(() => Promise.resolve(c.outcome()))
  const o = owner(false, "datapack")
  await tick()
  o.setKey("B")
  await tick()
  expect(o.actualProject().type).toBe("datapack")
  expect(c.dropdownMounts).toBe(1)
  const clicked = c.select(7)
  o.setKey("A")
  await tick()
  await clicked
  await tick()
  expectBInstall()
  expect(c.query).not.toHaveBeenCalledWith([
    "instance.checkShaderRequirements",
    7
  ])
  expect(c.success).toHaveBeenCalledWith(
    "notifications:_trn_datapack_downloaded:datapack-B",
    expect.anything()
  )
})
it("disposed wizard producer cannot publish its old callback task or feedback", async () => {
  const { read, clicked, o } = await clickBThenA()
  read.resolve(recommendation)
  await clicked
  o.dispose()
  c.open.mock.calls[0][1].onComplete(61)
  await tick()
  expect(c.query).not.toHaveBeenCalledWith(["vtask.getTaskOutcome", 61])
  expect(c.success).not.toHaveBeenCalled()
  expect(c.error).not.toHaveBeenCalled()
})
it("wizard completion without an accepted task produces no task feedback", async () => {
  const { read, clicked } = await clickBThenA()
  read.resolve(recommendation)
  await clicked
  c.open.mock.calls[0][1].onComplete(null)
  await tick()
  expect(c.query).not.toHaveBeenCalledWith(["vtask.getTaskOutcome", 61])
  expect(c.success).not.toHaveBeenCalled()
  expect(c.error).not.toHaveBeenCalled()
})
