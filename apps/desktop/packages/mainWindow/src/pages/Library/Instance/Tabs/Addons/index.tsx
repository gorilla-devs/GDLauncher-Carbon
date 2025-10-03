import { For, Show, createMemo, createSignal } from "solid-js"
import { useRouteData, useParams } from "@solidjs/router"
import fetchData from "../../instance.data"
import {
  AddonFilters,
  AddonTable,
  NoAddons,
  createAddonColumns
} from "./components"
import { useAddonData, useAddonMutations } from "./hooks"
const Addons = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData()
  const params = useParams()
  let tableInstance: any = null

  // Track filter height for dynamic table header offset
  const [filterHeight, setFilterHeight] = createSignal(0)

  // Data and state management
  const addonData = useAddonData()

  // Mutations and actions
  const addonMutations = useAddonMutations(addonData.allAddons.refetch, {
    optimisticToggleAddon: addonData.optimisticToggleAddon,
    optimisticDeleteAddon: addonData.optimisticDeleteAddon,
    optimisticDeleteAddons: addonData.optimisticDeleteAddons,
    optimisticUpdateAddon: addonData.optimisticUpdateAddon,
    rollbackToServerState: addonData.rollbackToServerState,
    startUpdatingMod: addonData.startUpdatingMod,
    stopUpdatingMod: addonData.stopUpdatingMod
  })

  const isInstanceLocked = () =>
    !!routeData.instanceDetails.data?.modpack?.locked

  const hasModloaders = () =>
    (routeData.instanceDetails.data?.modloaders?.length || 0) > 0

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

  // Calculate update counts
  const updateCount = createMemo(() => {
    return addonData.filteredAddons().filter((addon) => addon.has_update).length
  })

  // Calculate dynamic table header offset based on filter height
  const tableHeaderOffset = createMemo(() => {
    const baseTabHeight = 56 // top-14 = 56px
    const currentFilterHeight = filterHeight()

    // If no filter height measured yet, use default
    if (currentFilterHeight === 0) {
      return 170 // Default fallback
    }

    return baseTabHeight + currentFilterHeight
  })

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
    onDeleteMod: addonMutations.handleDeleteMod,
    isModUpdating: addonData.isModUpdating,
    instanceId: parseInt(params.id, 10)
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
        onAddAddons={() => addonMutations.gotoSearchPage()}
        onOpenFolder={addonMutations.handleOpenFolder}
        onUpdateAll={() =>
          addonMutations.handleUpdateAll(addonData.filteredAddons())
        }
        updateCount={updateCount}
        hasModloaders={hasModloaders}
        addons={() => addonData.addonsStore}
        onHeightChange={setFilterHeight}
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
            <NoAddons onAddAddons={() => addonMutations.gotoSearchPage()} />
          }
        >
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
              onTableReady={(table) => {
                tableInstance = table
              }}
              isInstanceLocked={isInstanceLocked}
              headerTopOffset={tableHeaderOffset()}
              mutations={{
                handleToggleMod: addonMutations.handleToggleMod,
                handleUpdateMod: addonMutations.handleUpdateMod,
                handleDeleteMod: addonMutations.handleDeleteMod,
                handleDeleteSelected: addonMutations.handleDeleteSelected,
                handleUpdateSelected: addonMutations.handleUpdateSelected,
                handleOpenFolder: addonMutations.handleOpenFolder,
                instanceId: parseInt(params.id, 10)
              }}
            />
          </div>
        </Show>
      </Show>
    </div>
  )
}

export default Addons
