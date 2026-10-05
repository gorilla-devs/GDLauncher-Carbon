import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import Changelog from "./Changelog"
const c = vi.hoisted(() => ({
  data: undefined as any,
  query: vi.fn(),
  select: undefined as any
}))
vi.mock("./changelog.data", () => ({ default: () => c.data }))
vi.mock("@solidjs/router", () => ({ useParams: () => ({ id: "1" }) }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: { useContext: () => ({ client: { query: c.query } }) }
}))
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => <span />
}))
vi.mock("@gd/ui", () => ({
  Select: (p: any) => {
    c.select = p
    return <div data-selection={p.value} data-options={p.options.join(",")} />
  },
  SelectTrigger: () => null,
  SelectContent: () => null,
  SelectItem: () => null,
  SelectValue: () => null
}))
let dispose: () => void
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.query.mockReset()
})
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
function fixture(details?: any, versions?: any, cf = false) {
  if (arguments.length === 0) {
    details = { id: "project" }
    versions = [{ id: "release", version_number: "1.0" }]
  }
  const [data, setData] = createSignal(versions)
  c.data = {
    isCurseforge: cf,
    modpackDetails: { data: details },
    modrinthProjectVersions: {
      get data() {
        return data()
      }
    }
  }
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <Changelog />, root)
  return { root, setData }
}
it("GPT-0126 empty releases clear selection/options and revoke delayed old changelog", async () => {
  let resolve!: (v: any) => void
  c.query.mockImplementation(() => new Promise((r) => (resolve = r)))
  const { root, setData } = fixture()
  await flush()
  expect(c.select.value).toBe("release")
  expect(c.select.options).toEqual(["release"])
  expect(() => setData([])).not.toThrow()
  await flush()
  expect(c.select.value).toBeUndefined()
  expect(c.select.options).toEqual([])
  expect(root.textContent).toContain("changelog_no_changelog")
  resolve({ changelog: "obsolete content", date_published: "2026-01-01" })
  await flush()
  expect(root.textContent).not.toContain("obsolete content")
  expect(c.query).toHaveBeenCalledTimes(1)
})
it.each([
  [undefined, []],
  [{ id: "project" }, undefined]
])(
  "missing details or pending versions stay harmless %j",
  async (details, versions) => {
    c.query.mockResolvedValue({})
    const { root } = fixture(details, versions)
    await flush()
    expect(root.textContent).toContain("changelog_no_changelog")
    expect(c.query).not.toHaveBeenCalled()
  }
)
it("CurseForge no files keeps harmless empty options", async () => {
  c.query.mockResolvedValue({ data: [], pagination: { totalCount: 0 } })
  const { root } = fixture(
    { data: { latestFiles: [], latestFilesIndexes: [] } },
    undefined,
    true
  )
  await flush()
  expect(c.select.options).toEqual([])
  expect(root.textContent).toContain("changelog_no_changelog")
})
