import { beforeEach, afterEach, it, expect, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import ExportCheckbox from "./atoms/ExportCheckbox"
import ExportCheckboxParent, {
  checkedFiles,
  setCheckedFiles
} from "./atoms/ExportCheckboxParent"
import { deferred, pointerGesture, settle } from "@/task32.fixture"
const c = vi.hoisted(() => ({
  query: vi.fn(),
  rootData: (() => undefined) as () => any
}))
vi.mock("@gd/ui", async () => import("@/task32.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { query: c.query } }),
    createQuery: () => ({
      get data() {
        return c.rootData()
      }
    })
  }
}))
let dispose: (() => void) | undefined
beforeEach(() => {
  c.query.mockReset()
  c.rootData = () => undefined
  setCheckedFiles([])
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  setCheckedFiles([])
})
function mount(initialData?: any[]) {
  const root = document.createElement("div")
  document.body.append(root)
  let setIdentity: any
  dispose = render(() => {
    const [identity, set] = createSignal({ instanceId: 7, path: ["folder"] })
    setIdentity = set
    return (
      <ExportCheckbox
        folder={{ name: "Folder", path: identity().path }}
        initialData={initialData}
        instanceId={identity().instanceId}
      />
    )
  }, root)
  return { root, setIdentity }
}
function expand(root: HTMLElement) {
  root.querySelector<HTMLElement>('[class*="arrow-down-bold"]')!.click()
}
it("0105 expanding while current directory request is pending retains and renders result", async () => {
  const wait = deferred()
  c.query.mockReturnValue(wait.promise)
  const { root } = mount()
  expect(c.query).toHaveBeenCalledTimes(1)
  expand(root)
  wait.resolve([{ name: "kept.txt", type: "File" }])
  await settle()
  expect(root.textContent).toContain("kept.txt")
  expect(c.query).toHaveBeenCalledTimes(1)
})
it("0105 collapse/reopen cannot revoke pending result or require another request", async () => {
  const wait = deferred()
  c.query.mockReturnValue(wait.promise)
  const { root } = mount()
  expand(root)
  expand(root)
  expand(root)
  wait.resolve([{ name: "kept.txt", type: "File" }])
  await settle()
  expect(root.textContent).toContain("kept.txt")
  expect(c.query).toHaveBeenCalledTimes(1)
})
it("0105 result received collapsed is retained for subsequent expansion", async () => {
  c.query.mockResolvedValue([{ name: "kept.txt", type: "File" }])
  const { root } = mount()
  await settle()
  expect(root.textContent).not.toContain("kept.txt")
  expand(root)
  expect(root.textContent).toContain("kept.txt")
})
it.each(["path", "instance"])(
  "0105 changed %s revokes old request and accepts only new owner",
  async (kind) => {
    const old = deferred(),
      current = deferred()
    c.query
      .mockReturnValueOnce(old.promise)
      .mockReturnValueOnce(current.promise)
    const { root, setIdentity } = mount()
    expand(root)
    setIdentity({
      instanceId: kind === "instance" ? 8 : 7,
      path: kind === "path" ? ["other"] : ["folder"]
    })
    current.resolve([{ name: "current.txt", type: "File" }])
    old.resolve([{ name: "stale.txt", type: "File" }])
    await settle()
    expect(root.textContent).toContain("current.txt")
    expect(root.textContent).not.toContain("stale.txt")
    expect(c.query).toHaveBeenCalledTimes(2)
  }
)
it("0105 disposed explorer accepts no late result", async () => {
  const wait = deferred()
  c.query.mockReturnValue(wait.promise)
  const { root } = mount()
  expand(root)
  dispose!()
  dispose = undefined
  wait.resolve([{ name: "late.txt", type: "File" }])
  await settle()
  expect(root.textContent).toBe("")
  expect(c.query).toHaveBeenCalledTimes(1)
})
it("0105 root initialData renders without redundant exploration and preserves folder/file subset selection", async () => {
  c.query.mockResolvedValue([])
  const { root } = mount([{ name: "kept.txt", type: "File" }])
  expect(root.textContent).toContain("kept.txt")
  expect(c.query).not.toHaveBeenCalled()
  const checkbox = root.querySelector<HTMLElement>('[class*="cursor-pointer"]')!
  pointerGesture(checkbox)
  await settle()
  expect(checkedFiles()).toContainEqual(["folder"])
  expect(checkedFiles()).toContainEqual(["folder", "kept.txt"])
  pointerGesture(checkbox)
  await settle()
  expect(checkedFiles()).toEqual([])
})

it("0105 actual parent owns loading unnamed root and later initialData/selection without a second explore", async () => {
  const root = document.createElement("div")
  document.body.append(root)
  const [data, set] = createSignal<any>()
  c.rootData = data
  dispose = render(() => <ExportCheckboxParent instanceId={7} />, root)
  expect(root.textContent).not.toContain("top.txt")
  expect(c.query).not.toHaveBeenCalled()
  set([{ name: "top.txt", type: "File" }])
  await settle()
  expect(root.textContent).toContain("top.txt")
  expect(c.query).not.toHaveBeenCalled()
  const checkbox = root
    .querySelector('input[type="checkbox"]')!
    .closest("label") as HTMLElement
  pointerGesture(checkbox)
  await settle()
  expect(checkedFiles()).toContainEqual(["top.txt"])
})
