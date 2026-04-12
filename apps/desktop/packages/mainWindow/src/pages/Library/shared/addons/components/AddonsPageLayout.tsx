import { For, Show, JSX, createSignal, createMemo, Accessor } from "solid-js"
import { ColumnDef } from "@tanstack/solid-table"
import { AddonTable } from "./AddonTable"
import { AddonFilters } from "./AddonFilters"
import { NoAddons } from "./NoAddons"
import type { AddonTableItem, AddonFiltersProps } from "../types"
import type { useAddonTableState } from "../hooks"

interface AddonsPageLayoutProps<T extends AddonTableItem> {
  isLoading: boolean
  filteredAddons: Accessor<T[]>
  filterProps: AddonFiltersProps
  stickyTop?: string
  columns: ColumnDef<any, any>[]
  tableState: ReturnType<typeof useAddonTableState>
  onTableReady?: (table: any) => void
  scrollContainerId: string
  contextMenuContent: (args: {
    selectedAddons: () => T[]
    selectionCount: () => number
  }) => JSX.Element
  onAddAddons: () => void
}

export function AddonsPageLayout<T extends AddonTableItem>(
  props: AddonsPageLayoutProps<T>
) {
  const [filterHeight, setFilterHeight] = createSignal(0)

  const tableHeaderOffset = createMemo(() => {
    const baseTabHeight = 80
    const currentFilterHeight = filterHeight()
    if (currentFilterHeight === 0) {
      return 200
    }
    return baseTabHeight + currentFilterHeight
  })

  return (
    <div class="flex flex-col">
      <AddonFilters
        {...props.filterProps}
        onHeightChange={setFilterHeight}
        stickyTop={props.stickyTop ?? "top-20"}
      />

      <Show when={props.isLoading}>
        <div class="p-6">
          <div class="animate-pulse space-y-4">
            <For each={Array(5).fill(0)}>
              {() => <div class="bg-darkSlate-700 h-12 rounded" />}
            </For>
          </div>
        </div>
      </Show>

      <Show when={!props.isLoading}>
        <Show
          when={props.filteredAddons().length > 0}
          fallback={<NoAddons onAddAddons={props.onAddAddons} />}
        >
          <div class="px-6 pb-6">
            <AddonTable
              data={props.filteredAddons}
              columns={props.columns}
              sorting={props.tableState.sorting}
              setSorting={props.tableState.setSorting}
              columnFilters={props.tableState.columnFilters}
              setColumnFilters={props.tableState.setColumnFilters}
              columnVisibility={props.tableState.columnVisibility}
              setColumnVisibility={props.tableState.setColumnVisibility}
              rowSelection={props.tableState.rowSelection}
              setRowSelection={props.tableState.setRowSelection}
              onTableReady={props.onTableReady}
              headerTopOffset={tableHeaderOffset()}
              scrollContainerId={props.scrollContainerId}
              contextMenuContent={props.contextMenuContent}
            />
          </div>
        </Show>
      </Show>
    </div>
  )
}
