import { Show, createMemo } from "solid-js"
import { ContextMenuItem, ContextMenuSeparator } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { useParams } from "@solidjs/router"
import {
  AddonsPageLayout,
  createAddonColumns
} from "@/pages/Library/shared/addons/components"
import { getServerModImageUrl } from "@/utils/instances"
import { useServerAddonData, useServerAddonMutations } from "./hooks"

const ServerAddons = () => {
  const [t] = useTransContext()
  const params = useParams()
  let tableInstance: any = null

  const addonData = useServerAddonData()

  const addonMutations = useServerAddonMutations(
    addonData.allAddons.refetch,
    {
      optimisticToggleAddon: addonData.optimisticToggleAddon,
      optimisticDeleteAddon: addonData.optimisticDeleteAddon,
      optimisticDeleteAddons: addonData.optimisticDeleteAddons,
      rollbackToServerState: addonData.rollbackToServerState
    },
    addonData.setRowSelection
  )

  const selectedRows = createMemo(() => {
    const rowSelectionState = addonData.rowSelection()
    const selectedIds = Object.keys(rowSelectionState).filter(
      (id) => rowSelectionState[id]
    )
    return addonData
      .filteredAddons()
      .filter((addon) => selectedIds.includes(addon.id))
  })

  const columns = createAddonColumns({
    selectedCount: () => selectedRows().length,
    totalRows: () => addonData.filteredAddons().length,
    onSelectAll: () => {
      if (!tableInstance) return
      const totalRows = addonData.filteredAddons().length
      const selectedCount = selectedRows().length
      if (selectedCount === totalRows && totalRows > 0) {
        tableInstance.toggleAllRowsSelected(false)
      } else {
        tableInstance.toggleAllRowsSelected(true)
      }
    },
    onToggleMod: addonMutations.handleToggleMod,
    onDeleteMod: addonMutations.handleDeleteMod,
    getDisplayName: (row) => row.displayName || row.filename,
    getSubtitle: (row) =>
      row.displayName && row.displayName !== row.filename ? row.filename : null,
    getAddonType: (row) => row.addonType,
    getImageUrl: (row) => {
      if (!row.hasImage) return null
      return getServerModImageUrl(params.id!, row.id, "metadata")
    },
    getPlatformInfo: (row) => ({
      hasCurseforge: !!row.curseforgeProjectId,
      hasModrinth: !!row.modrinthProjectId
    })
  })

  return (
    <AddonsPageLayout
      isLoading={addonData.allAddons.isLoading}
      filteredAddons={addonData.filteredAddons}
      filterProps={{
        searchQuery: addonData.searchQuery,
        setSearchQuery: addonData.setSearchQuery,
        enabledAddonTypes: addonData.enabledAddonTypes,
        setEnabledAddonTypes: addonData.setEnabledAddonTypes,
        addonTypes: () =>
          ["mods", "datapacks"].filter((t) =>
            addonData.addonsStore.some((a) => a.addonType === t)
          ),
        onAddAddons: () => addonMutations.gotoSearchPage(),
        onOpenFolder: addonMutations.handleOpenFolder
      }}
      columns={columns}
      tableState={addonData}
      onTableReady={(t) => {
        tableInstance = t
      }}
      scrollContainerId="main-container-server-details"
      onAddAddons={() => addonMutations.gotoSearchPage()}
      contextMenuContent={({ selectedAddons, selectionCount }) => (
        <Show
          when={selectionCount() === 1}
          fallback={
            <>
              <ContextMenuItem disabled>
                {t("content:_trn_selected_count", {
                  count: selectionCount()
                })}
              </ContextMenuItem>
              <ContextMenuSeparator />
              <Show when={selectedAddons().some((a) => !a.enabled)}>
                <ContextMenuItem
                  onSelect={() => {
                    selectedAddons()
                      .filter((a) => !a.enabled)
                      .forEach((a) => addonMutations.handleToggleMod(a))
                  }}
                >
                  <div class="i-hugeicons:toggle-off mr-2" />
                  {t("content:_trn_enable_all")}
                </ContextMenuItem>
              </Show>
              <Show when={selectedAddons().some((a) => a.enabled)}>
                <ContextMenuItem
                  onSelect={() => {
                    selectedAddons()
                      .filter((a) => a.enabled)
                      .forEach((a) => addonMutations.handleToggleMod(a))
                  }}
                >
                  <div class="i-hugeicons:toggle-off mr-2" />
                  {t("content:_trn_disable_all")}
                </ContextMenuItem>
              </Show>
              <ContextMenuSeparator />
              <ContextMenuItem
                onSelect={() =>
                  addonMutations.handleDeleteSelected(selectedAddons())
                }
                class="text-red-400 focus:text-red-400"
              >
                <div class="i-hugeicons:delete-02 mr-2" />
                {t("content:_trn_delete_selected")}
              </ContextMenuItem>
            </>
          }
        >
          <Show when={selectedAddons()[0]}>
            {(addon) => {
              const displayName = () => addon().displayName || addon().filename
              return (
                <>
                  <ContextMenuItem
                    onSelect={() => {
                      window.navigator.clipboard.writeText(displayName())
                    }}
                  >
                    <div class="i-hugeicons:clipboard mr-2" />
                    {t("content:_trn_copy_name")}
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    onSelect={() => addonMutations.handleToggleMod(addon())}
                  >
                    <div class="i-hugeicons:toggle-off mr-2" />
                    {addon().enabled
                      ? t("content:_trn_disable_mod")
                      : t("content:_trn_enable_mod")}
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    onSelect={() => addonMutations.handleOpenFolder()}
                  >
                    <div class="i-hugeicons:folder-open mr-2" />
                    {t("instances:_trn_open_folder")}
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    onSelect={() => addonMutations.handleDeleteMod(addon())}
                    class="text-red-400 focus:text-red-300"
                  >
                    <div class="i-hugeicons:delete-02 mr-2" />
                    {t("content:_trn_delete_mod")}
                  </ContextMenuItem>
                </>
              )
            }}
          </Show>
        </Show>
      )}
    />
  )
}

export default ServerAddons
