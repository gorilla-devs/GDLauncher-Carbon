import { For, Show, createMemo } from "solid-js"
import { useRouteData } from "@solidjs/router"
import fetchData from "../../instance.data"
import {
  AddonFilters,
  AddonTable,
  BulkActions,
  NoAddons,
  createAddonColumns
} from "./components"
import { useAddonData, useAddonMutations } from "./hooks"
const Addons = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  let tableInstance: any = null

  // Data and state management
  const addonData = useAddonData()

  // Mutations and actions
  const addonMutations = useAddonMutations(addonData.allAddons.refetch, {
    optimisticToggleAddon: addonData.optimisticToggleAddon,
    optimisticDeleteAddon: addonData.optimisticDeleteAddon,
    optimisticDeleteAddons: addonData.optimisticDeleteAddons,
    rollbackToServerState: addonData.rollbackToServerState
  })

  const isInstanceLocked = () =>
    !!routeData.instanceDetails.data?.modpack?.locked

  // Get selected rows reactively using row selection state
  const selectedRows = createMemo(() => {
    const rowSelectionState = addonData.rowSelection()
    const selectedIds = Object.keys(rowSelectionState).filter(
      (id) => rowSelectionState[id]
    )
    return addonData
      .filteredAddons()
      .filter((addon) => selectedIds.includes(addon.id))
  })

  const getSelectedRows = () => selectedRows()

  // Column configuration
  const columns = createAddonColumns({
    isInstanceLocked,
    selectedCount: () => getSelectedRows().length,
    totalRows: () => addonData.filteredAddons().length,
    onSelectAll: () => {
      if (!tableInstance) return

      const totalRows = addonData.filteredAddons().length
      const selectedCount = getSelectedRows().length

      if (selectedCount === totalRows && totalRows > 0) {
        // All are selected, so deselect all
        tableInstance.toggleAllRowsSelected(false)
      } else {
        // None or some are selected, so select all
        tableInstance.toggleAllRowsSelected(true)
      }
    },
    onToggleMod: addonMutations.handleToggleMod,
    onUpdateMod: addonMutations.handleUpdateMod,
    onDeleteMod: addonMutations.handleDeleteMod
  })

  return (
    <div class="flex flex-col">
      {/* Filters - Sticky */}
      <AddonFilters
        searchQuery={addonData.searchQuery}
        setSearchQuery={addonData.setSearchQuery}
        enabledAddonTypes={addonData.enabledAddonTypes}
        setEnabledAddonTypes={addonData.setEnabledAddonTypes}
        platformFilter={addonData.platformFilter}
        setPlatformFilter={addonData.setPlatformFilter}
        isInstanceLocked={isInstanceLocked}
        onAddAddons={() => addonMutations.gotoSearchPage("mods")}
        onOpenFolder={addonMutations.handleOpenFolder}
      />

      {/* Loading state */}
      <Show when={addonData.allAddons.isLoading}>
        <div class="p-6">
          <div class="animate-pulse space-y-4">
            <For each={Array(5).fill(0)}>
              {() => <div class="bg-darkSlate-700 h-12 rounded" />}
            </For>
          </div>
        </div>
      </Show>

      {/* Table content - only show when not loading */}
      <Show when={!addonData.allAddons.isLoading}>
        <Show
          when={addonData.filteredAddons().length > 0}
          fallback={
            <NoAddons
              onAddAddons={() => addonMutations.gotoSearchPage("mods")}
            />
          }
        >
          {/* Sticky BulkActions Container */}
          <div class="sticky top-[115px] z-[12] bg-darkSlate-800 px-6">
            <BulkActions
              class="pt-4"
              selectedRowsLength={() => getSelectedRows().length}
              isInstanceLocked={isInstanceLocked}
              onDeleteSelected={async () => {
                const selectedRows = getSelectedRows()
                await addonMutations.handleDeleteSelected(selectedRows)
                if (tableInstance) {
                  tableInstance.toggleAllRowsSelected(false)
                }
              }}
              onClearSelection={() => {
                if (tableInstance) {
                  tableInstance.toggleAllRowsSelected(false)
                }
              }}
            />
          </div>

          {/* Table */}
          <div class="px-6 pb-6">
            <AddonTable
              data={addonData.filteredAddons}
              columns={columns}
              sorting={addonData.sorting}
              setSorting={addonData.setSorting}
              columnFilters={addonData.columnFilters}
              setColumnFilters={addonData.setColumnFilters}
              columnVisibility={addonData.columnVisibility}
              setColumnVisibility={addonData.setColumnVisibility}
              rowSelection={addonData.rowSelection}
              setRowSelection={addonData.setRowSelection}
              hasBulkActions={getSelectedRows().length > 0}
              onTableReady={(table) => {
                tableInstance = table
              }}
            />
          </div>
        </Show>
      </Show>
    </div>
  )
}

export default Addons
