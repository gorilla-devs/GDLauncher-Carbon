import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import ModpackDownloadButton from "./ModpackDownloadButton"

const c = vi.hoisted(() => ({
  query: vi.fn(),
  creates: [] as any[],
  callbacks: {} as Record<string, any>,
  errors: [] as string[],
  navigate: vi.fn()
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { query: c.query } }),
    createMutation: (options: any) => {
      const opts = options()
      c.callbacks[opts.mutationKey[0]] = opts
      return {
        mutate: (input: any) =>
          c.creates.push({ key: opts.mutationKey[0], input })
      }
    }
  }
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../packages/ui/src/Button")),
  Spinner: () => <span>loading</span>,
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div>{p.children}</div>,
  toast: { success: vi.fn(), error: (error: string) => c.errors.push(error) }
}))
let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  c.query.mockReset()
  c.creates = []
  c.callbacks = {}
  c.errors = []
  c.navigate.mockReset()
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
function fixture(
  platform = "modrinth",
  fileId?: string,
  mainFileId: string | null = "historical"
) {
  const [addon, setAddon] = createSignal<any>({
    platform,
    id: platform === "curseforge" ? "123" : "project-A",
    title: "Pack A",
    imageUrl: "synthetic-A.png",
    mainFileId
  })
  const [name, setName] = createSignal("Chosen A")
  const [file, setFile] = createSignal(fileId)
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <ModpackDownloadButton addon={addon()} name={name()} fileId={file()} />
    ),
    root
  )
  return { button: root.querySelector("button")!, setAddon, setName, setFile }
}
it("D3 captures project, name, image and file inputs before latest lookup settles", async () => {
  let resolve!: (versions: any) => void
  c.query.mockReturnValue(
    new Promise((r) => {
      resolve = r
    })
  )
  const { button, setAddon, setName, setFile } = fixture()
  button.click()
  expect(button.disabled).toBe(true)
  setAddon({
    platform: "modrinth",
    id: "project-B",
    title: "Pack B",
    imageUrl: "synthetic-B.png"
  })
  setName("Chosen B")
  setFile("explicit-B")
  resolve([{ id: "latest-A" }])
  await flush()
  expect(c.creates).toEqual([
    {
      key: "instance.createInstance",
      input: {
        use_loaded_icon: false,
        icon_url: "synthetic-A.png",
        notes: "",
        name: "Chosen A",
        version: {
          Modpack: {
            type: "modrinth",
            value: { project_id: "project-A", version_id: "latest-A" }
          }
        }
      }
    }
  ])
})
it.each(["rejected", "empty"])(
  "Task21 carrier: same button retries %s lookup without creation",
  async (kind) => {
    if (kind === "rejected")
      c.query.mockRejectedValue(new Error("synthetic lookup failed"))
    else c.query.mockResolvedValue([])
    const { button } = fixture()
    button.click()
    await flush()
    expect(c.creates).toEqual([])
    expect(c.errors).toHaveLength(1)
    expect(button.disabled).toBe(false)
    c.query.mockResolvedValue([{ id: "recovered" }])
    button.click()
    await flush()
    expect(c.creates[0].input.version.Modpack.value.version_id).toBe(
      "recovered"
    )
    expect(button.disabled).toBe(true)
  }
)
it("Task21 carrier: missing CurseForge main file reports error and same button can retry explicit selection", async () => {
  const { button, setFile } = fixture("curseforge", undefined, null)
  button.click()
  await flush()
  expect(c.creates).toEqual([])
  expect(c.errors).toHaveLength(1)
  expect(button.disabled).toBe(false)
  setFile("456")
  button.click()
  await flush()
  expect(c.query).not.toHaveBeenCalled()
  expect(c.creates[0].input.version.Modpack).toEqual({
    type: "curseforge",
    value: { project_id: 123, file_id: 456 }
  })
})
it.each(["success", "error"])(
  "accepted create retains loading through prepare %s settlement",
  async (outcome) => {
    const { button } = fixture("modrinth", "explicit", null)
    button.click()
    button.click()
    await flush()
    expect(c.query).not.toHaveBeenCalled()
    expect(c.creates).toHaveLength(1)
    expect(button.disabled).toBe(true)
    c.callbacks["instance.createInstance"].onSuccess(42)
    expect(c.creates[1]).toEqual({ key: "instance.prepareInstance", input: 42 })
    expect(button.disabled).toBe(true)
    button.click()
    expect(c.creates).toHaveLength(2)
    if (outcome === "success")
      await c.callbacks["instance.prepareInstance"].onSuccess()
    else c.callbacks["instance.prepareInstance"].onError()
    expect(button.disabled).toBe(false)
    expect(c.navigate).toHaveBeenCalledTimes(outcome === "success" ? 1 : 0)
  }
)
it("accepted create failure releases the originating button for retry", async () => {
  const { button } = fixture("modrinth", "explicit")
  button.click()
  await flush()
  c.callbacks["instance.createInstance"].onError()
  expect(button.disabled).toBe(false)
  expect(c.errors).toHaveLength(1)
  button.click()
  await flush()
  expect(c.creates).toHaveLength(2)
})
