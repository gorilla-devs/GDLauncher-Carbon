import { createSignal } from "solid-js"
import {
  SortingState,
  ColumnFiltersState,
  VisibilityState,
  RowSelectionState
} from "@tanstack/solid-table"

export const useAddonTableState = () => {
  const [sorting, setSorting] = createSignal<SortingState>([
    { id: "filename", desc: false }
  ])
  const [columnFilters, setColumnFilters] = createSignal<ColumnFiltersState>([])
  const [columnVisibility, setColumnVisibility] = createSignal<VisibilityState>(
    {}
  )
  const [rowSelection, setRowSelection] = createSignal<RowSelectionState>({})

  return {
    sorting,
    setSorting,
    columnFilters,
    setColumnFilters,
    columnVisibility,
    setColumnVisibility,
    rowSelection,
    setRowSelection
  }
}
