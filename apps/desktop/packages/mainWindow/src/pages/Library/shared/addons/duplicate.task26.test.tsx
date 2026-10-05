import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore, reconcile } from "solid-js/store"
import { createSolidTable, getCoreRowModel } from "@tanstack/solid-table"
vi.mock("@gd/ui", async () => await import("@/test/task26/ui"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@gd/i18n/helpers", () => ({ getPlatformKey: (s: string) => s }))
import { createAddonColumns } from "./components/columns/addonColumns"
let dispose: (() => void) | undefined, root: HTMLDivElement
const config = {
  selectedCount: () => 0,
  totalRows: () => 2,
  onSelectAll: vi.fn(),
  onToggleMod: vi.fn(),
  onDeleteMod: vi.fn(),
  getDisplayName: (r: any) => r.id,
  getAddonType: () => "mod",
  isDuplicate: (r: any) => r.is_duplicate
}
afterEach(() => {
  dispose?.()
  root?.remove()
})
test("GPT0148 mounted duplicate cell follows reconciled stable TanStack row metadata in both directions", () => {
  const [rows, setRows] = createStore([
    { id: "one", enabled: true, is_duplicate: true },
    { id: "two", enabled: true, is_duplicate: true }
  ])
  let table: any, row: any, cell: any
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => {
    const columns = createAddonColumns({
      ...config,
      onToggleMod: (r: any) =>
        setRows(
          reconcile(
            [
              { id: "one", enabled: true, is_duplicate: !r.enabled },
              { id: "two", enabled: !r.enabled, is_duplicate: !r.enabled }
            ],
            { key: "id" }
          )
        )
    })
    table = createSolidTable({
      data: rows,
      columns,
      getRowId: (r) => r.id,
      getCoreRowModel: getCoreRowModel()
    })
    row = table.getRowModel().rows[0]
    cell = row.getAllCells().find((c: any) => c.column.id === "duplicate")
    const toggle = table
      .getRowModel()
      .rows[1].getAllCells()
      .find((c: any) => c.column.id === "enabled")
    return (
      <>
        {cell.column.columnDef.cell(cell.getContext())}
        {toggle.column.columnDef.cell(toggle.getContext())}
      </>
    )
  }, root)
  expect(root.textContent).toContain("_trn_duplicate_mod_warning")
  root
    .querySelector("input")!
    .dispatchEvent(new Event("change", { bubbles: true }))
  expect(table.getRowModel().rows[0]).toBe(row)
  expect(row.getAllCells().find((c: any) => c.column.id === "duplicate")).toBe(
    cell
  )
  expect(row.original.is_duplicate).toBe(false)
  expect(root.textContent).not.toContain("_trn_duplicate_mod_warning")
  root
    .querySelector("input")!
    .dispatchEvent(new Event("change", { bubbles: true }))
  expect(root.textContent).toContain("_trn_duplicate_mod_warning")
  setRows(
    reconcile([{ id: "one", enabled: true, is_duplicate: false }], {
      key: "id"
    })
  )
  expect(root.textContent).not.toContain("_trn_duplicate_mod_warning")
})
test("duplicate cell dereferences a replaced current row.original instead of keeping the first row snapshot", () => {
  const [context, setContext] = createStore({
    row: { original: { id: "one", is_duplicate: true } }
  })
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => {
    const col = createAddonColumns(config).find((col) => col.id === "duplicate")
    return col.cell(context)
  }, root)
  expect(root.textContent).toContain("_trn_duplicate_mod_warning")
  setContext("row", "original", { id: "two", is_duplicate: false })
  expect(root.textContent).not.toContain("_trn_duplicate_mod_warning")
})
