import { Accessor, JSX } from "solid-js"
import {
  ColumnDef,
  SortingState,
  ColumnFiltersState,
  VisibilityState,
  RowSelectionState
} from "@tanstack/solid-table"

/**
 * Minimal interface that both Mod and ServerAddon satisfy.
 * The shared table only needs these fields for row identity and selection.
 */
export interface AddonTableItem {
  id: string
  filename: string
  enabled: boolean
}

export interface AddonTableProps<T extends AddonTableItem> {
  data: Accessor<T[]>
  columns: ColumnDef<any, any>[]
  sorting: Accessor<SortingState>
  setSorting: (
    sorting: SortingState | ((prev: SortingState) => SortingState)
  ) => void
  columnFilters: Accessor<ColumnFiltersState>
  setColumnFilters: (
    filters:
      | ColumnFiltersState
      | ((prev: ColumnFiltersState) => ColumnFiltersState)
  ) => void
  columnVisibility: Accessor<VisibilityState>
  setColumnVisibility: (
    visibility: VisibilityState | ((prev: VisibilityState) => VisibilityState)
  ) => void
  rowSelection: Accessor<RowSelectionState>
  setRowSelection: (
    selection:
      | RowSelectionState
      | ((prev: RowSelectionState) => RowSelectionState)
  ) => void
  onTableReady?: (table: any) => void
  scrollContainerId?: string
  headerTopOffset?: number
  contextMenuContent: (args: {
    selectedAddons: () => T[]
    selectionCount: () => number
  }) => JSX.Element
}

export interface AddonFiltersProps {
  searchQuery: () => string
  setSearchQuery: (q: string) => void
  enabledAddonTypes: Record<string, boolean>
  setEnabledAddonTypes: (type: string, enabled: boolean) => void
  addonTypes: () => string[]
  onAddAddons: () => void
  onOpenFolder: () => void
  onHeightChange?: (height: number) => void
  stickyTop?: string
  searchInputClass?: string
  extraActions?: JSX.Element
  addButtonDisabled?: boolean
  addButtonTooltip?: JSX.Element
}
