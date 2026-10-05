import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import { AddonTable } from "./components/AddonTable"
vi.mock("@gd/ui", async () => await import("@/test/task26/ui"))
let root: HTMLDivElement,
  owner: HTMLDivElement,
  dispose: (() => void) | undefined,
  table: any,
  setData: any,
  setFilters: any,
  setSorting: any,
  selection: any
let viewportHeight: number
let measuredOwner: HTMLElement
let resize: ResizeObserverCallback
let observers: { fn: ResizeObserverCallback; target?: Element }[]
beforeEach(() => {
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn(() => 1)
  )
  vi.stubGlobal("cancelAnimationFrame", vi.fn())
  observers = []
  viewportHeight = 600
  vi.stubGlobal(
    "ResizeObserver",
    class {
      row: { fn: ResizeObserverCallback; target?: Element }
      constructor(fn: ResizeObserverCallback) {
        resize = fn
        this.row = { fn }
        observers.push(this.row)
      }
      observe(target: Element) {
        this.row.target = target
      }
      disconnect() {}
    }
  )
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    function (this: HTMLElement) {
      if (this.classList.contains("sticky")) return new DOMRect(0, 160, 500, 40)
      if (
        this.children[0]?.getAttribute("style")?.includes("will-change: height")
      )
        return new DOMRect(0, 500 - measuredOwner.scrollTop, 500, 30000)
      if (this === document.documentElement)
        return new DOMRect(0, -this.scrollTop, 500, 30000)
      return new DOMRect(0, 0, 500, viewportHeight)
    }
  )
  owner = document.createElement("div")
  owner.id = "fixture-scroll"
  owner.style.overflow = "auto"
  Object.defineProperty(owner, "clientHeight", { get: () => viewportHeight })
  measuredOwner = owner
  owner.scrollTop = 20300
  root = document.createElement("div")
  owner.append(root)
  document.body.append(owner)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  owner.remove()
  document.documentElement.scrollTop = 0
  delete (document.documentElement as any).clientHeight
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})
const all = Array.from({ length: 500 }, (_, i) => ({
  id: String(i),
  filename: i < 3 ? `match-${i}` : `other-${i}`,
  enabled: true
}))
function mount(explicit = true, dynamic = false, measured = false) {
  const [data, update] = createSignal(all)
  setData = update
  const [filters, filter] = createSignal<any[]>([])
  setFilters = filter
  const [sort, sorting] = createSignal<any[]>([])
  setSorting = sorting
  const [selected, select] = createSignal<any>({})
  selection = selected
  dispose = render(
    () => (
      <AddonTable
        data={data}
        columns={[
          {
            accessorKey: "filename",
            header: "Name",
            cell: (p: any) => p.getValue()
          }
        ]}
        sorting={sort}
        setSorting={sorting}
        columnFilters={filters}
        setColumnFilters={filter}
        columnVisibility={() => ({})}
        setColumnVisibility={vi.fn()}
        rowSelection={selected}
        setRowSelection={select}
        contextMenuContent={() => null}
        scrollContainerId={explicit ? owner.id : undefined}
        onTableReady={(v) => (table = v)}
        virtualizationConfig={{
          enableDynamicHeight: dynamic,
          rowHeight: dynamic && !measured ? () => 80 : 60
        }}
      />
    ),
    root
  )
}
const ids = () =>
  [...root.querySelectorAll("[data-row-id]")].map((el) =>
    el.getAttribute("data-row-id")
  )
const spacers = () =>
  [...root.querySelectorAll<HTMLElement>('[style*="will-change: height"]')].map(
    (el) => parseFloat(el.style.height)
  )
test.each([true, false])(
  "%s explicit/discovered owner renders three filtered rows and reconciles page origin without native clamp feedback",
  (explicit) => {
    mount(explicit)
    expect(ids().length).toBeGreaterThan(0)
    expect(Number(ids()[0])).toBeGreaterThan(300)
    setFilters([{ id: "filename", value: "match" }])
    expect(ids()).toEqual(["0", "1", "2"])
    expect(spacers()).toEqual([0, 0])
    expect(owner.scrollTop).toBe(300)
    owner.dispatchEvent(new Event("scroll"))
    expect(ids()).toEqual(["0", "1", "2"])
  }
)
test("shrink stays scrollable at its valid tail; resize reconciles again", () => {
  mount()
  setData(all.slice(0, 30))
  expect(ids().length).toBeGreaterThan(0)
  expect(ids().at(-1)).toBe("29")
  expect(owner.scrollTop).toBe(1700)
  expect(500 - owner.scrollTop + 1800).toBe(600)
  expect(spacers()[0]).toBeLessThan(1800)
  expect(spacers()[1]).toBe(0)
  viewportHeight = 2000
  resize([{ target: owner, contentRect: { height: 2000 } } as any], {} as any)
  expect(owner.scrollTop).toBe(300)
  expect(ids()).toHaveLength(30)
})
test("zero model has zero spacers and offset, then reset renders healthy rows", () => {
  mount()
  setData([])
  expect(ids()).toEqual([])
  expect(spacers()).toEqual([0, 0])
  expect(owner.scrollTop).toBe(300)
  setData(all.slice(0, 3))
  expect(ids()).toEqual(["0", "1", "2"])
})
test("dynamic callback heights bound tail and retain sorted row/selection identities", () => {
  mount(true, true)
  setData(all.slice(0, 30))
  expect(ids().at(-1)).toBe("29")
  expect(owner.scrollTop).toBe(2300)
  expect(spacers()[0] + ids().length * 80 + spacers()[1]).toBe(2400)
  setFilters([{ id: "filename", value: "match" }])
  setSorting([{ id: "filename", desc: true }])
  expect(ids()).toEqual(["2", "1", "0"])
  expect(spacers()).toEqual([0, 0])
  expect(owner.scrollTop).toBe(300)
  const row = root.querySelector('[data-row-id="1"]')!
  row.dispatchEvent(new MouseEvent("mousedown", { button: 0, bubbles: true }))
  document.dispatchEvent(new MouseEvent("mouseup"))
  expect(selection()).toEqual({ 1: true })
  expect(table.getRowModel().rows.map((r: any) => r.id)).toEqual([
    "2",
    "1",
    "0"
  ])
})

test("measured dynamic heights contribute to total and spacer bounds after shrink", () => {
  mount(true, true, true)
  setData(all.slice(0, 30))
  const observer = observers.findLast(
    (item) => item.target?.getAttribute("data-row-id") === "25"
  )!
  expect(observer).toBeDefined()
  observer.fn(
    [{ target: observer.target, contentRect: { height: 120 } } as any],
    {} as any
  )
  owner.scrollTop = 20300
  owner.dispatchEvent(new Event("scroll"))
  expect(owner.scrollTop).toBe(1760)
  expect(ids().at(-1)).toBe("29")
  expect(spacers()[0]).toBeGreaterThanOrEqual(0)
  expect(spacers()[1]).toBe(0)
  setData([])
  expect(spacers()).toEqual([0, 0])
  expect(owner.scrollTop).toBe(300)
})

test("ordinary large-table body end remains reachable below an occupied sticky header", () => {
  mount()
  // Owner bottom=600, sticky header bottom=200: the body has 400px available.
  // Page body origin=300 and content=30000, so native 29900 places its end at600.
  owner.scrollTop = 29900
  owner.dispatchEvent(new Event("scroll"))
  expect(ids().at(-1)).toBe("499")
  expect(owner.scrollTop).toBe(29900)
  const bodyTop = 500 - measuredOwner.scrollTop
  expect(bodyTop + 30000).toBeLessThanOrEqual(
    owner.getBoundingClientRect().bottom
  )
})

test("document fallback uses the client viewport rather than the entire scrolled content rectangle", () => {
  owner.style.overflow = "visible"
  measuredOwner = document.documentElement
  Object.defineProperty(measuredOwner, "clientHeight", {
    configurable: true,
    get: () => viewportHeight
  })
  measuredOwner.scrollTop = 20300
  mount(false)
  measuredOwner.scrollTop = 29900
  measuredOwner.dispatchEvent(new Event("scroll"))
  expect(measuredOwner.scrollTop).toBe(29900)
  expect(ids().at(-1)).toBe("499")
  setFilters([{ id: "filename", value: "match" }])
  expect(measuredOwner.scrollTop).toBe(300)
  expect(ids()).toEqual(["0", "1", "2"])
})

test("draws rows for the whole viewport while the measured body area is still empty", () => {
  vi.mocked(HTMLElement.prototype.getBoundingClientRect).mockImplementation(
    function (this: HTMLElement) {
      // The sticky header ends at the viewport bottom, so no body is on
      // screen when the table measures; content above may move it up later.
      if (this.classList.contains("sticky")) return new DOMRect(0, 560, 500, 40)
      if (
        this.children[0]?.getAttribute("style")?.includes("will-change: height")
      )
        return new DOMRect(0, 500 - owner.scrollTop, 500, 30000)
      return new DOMRect(0, 0, 500, viewportHeight)
    }
  )
  owner.scrollTop = 0
  mount()
  // 600px of viewport holds 10 rows of 60px; the buffer adds more.
  expect(ids().length).toBeGreaterThanOrEqual(12)
})
