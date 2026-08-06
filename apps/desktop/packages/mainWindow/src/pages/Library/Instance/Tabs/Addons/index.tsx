import { Show, createMemo } from "solid-js"
import {
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubTrigger,
  ContextMenuSubContent,
  ContextMenuPortal,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Button,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { getViewOnKey } from "@gd/i18n/helpers"
import { toast } from "@gd/ui"
import { useParams } from "@solidjs/router"
import { AddonType, Mod as ModType } from "@gd/core_module/bindings"
import {
  AddonsPageLayout,
  createAddonColumns
} from "@/pages/Library/shared/addons/components"
import { supportsEnableToggle } from "@/pages/Library/shared/addons/addonCapabilities"
import { useModal } from "@/managers/ModalsManager"
import { useGDNavigate } from "@/managers/NavigationManager"
import { getModImageUrl } from "@/utils/instances"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import useInstanceData from "../../instance.data"
import { useAddonData, useAddonMutations } from "./hooks"

const ADDON_TYPES: AddonType[] = [
  "mods",
  "shaders",
  "resourcepacks",
  "datapacks",
  "worlds"
]

const Addons = () => {
  const [t] = useTransContext()
  const routeData = useInstanceData()
  const params = useParams<{ id: string }>()
  const modalsContext = useModal()
  const navigator = useGDNavigate()
  let tableInstance: any = null

  const addonData = useAddonData()

  const addonMutations = useAddonMutations(
    addonData.allAddons.refetch,
    {
      optimisticToggleAddon: addonData.optimisticToggleAddon,
      optimisticDeleteAddon: addonData.optimisticDeleteAddon,
      optimisticDeleteAddons: addonData.optimisticDeleteAddons,
      optimisticUpdateAddon: addonData.optimisticUpdateAddon,
      rollbackToServerState: addonData.rollbackToServerState,
      startUpdatingMod: addonData.startUpdatingMod,
      stopUpdatingMod: addonData.stopUpdatingMod
    },
    addonData.setRowSelection
  )

  const isInstanceLocked = () =>
    !!routeData.instanceDetails.data?.modpack?.locked

  /**
   * Whether this row's addon type can be enabled/disabled at all.
   *
   * Worlds cannot (`supportsEnableToggle`): disabling renames the addon to
   * `<name>.disabled`, and for a save — a directory — that neither hides it
   * from Minecraft nor from our own scanner, which re-reports the renamed
   * directory as a *new, enabled* world. Every route to the toggle has to
   * respect that, not just the table's own column: a world dragged into a
   * multi-selection would otherwise be swept up by "Disable all", and the
   * row's own context menu offered it outright.
   */
  const canToggle = (addon: ModType) => supportsEnableToggle(addon.addon_type)

  const hasModloaders = () =>
    (routeData.instanceDetails.data?.modloaders?.length || 0) > 0

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

  const updateCount = createMemo(() => {
    return addonData.filteredAddons().filter((addon) => addon.has_update).length
  })

  const visibleAddonTypes = () => {
    return ADDON_TYPES.filter((type) => {
      if (type === "mods" && !hasModloaders()) {
        return false
      }
      const hasAddonsOfType = addonData.addonsStore.some(
        (addon) => addon.addon_type === type
      )
      return hasAddonsOfType
    })
  }

  const instanceId = () => parseInt(params.id, 10)

  // If the user has filtered the view to exactly one addon type, that's
  // the type they're interested in adding next; otherwise default to
  // "mods" — the most common case across instances. Without an explicit
  // type, gotoSearchPage emits an empty :type segment in the URL and the
  // search page falls through to whatever projectType the search context
  // last held (often "modpack"), which is the bug we're fixing.
  const defaultSearchType = (): AddonType => {
    const enabled = (
      Object.keys(addonData.enabledAddonTypes) as AddonType[]
    ).filter((t) => addonData.enabledAddonTypes[t])
    if (enabled.length === 1) return enabled[0]
    // `hasModloaders()` reads a field off `instanceDetails.data`, so it is
    // false in two different situations: the instance genuinely has no
    // modloaders, and the query has not resolved yet. Only the first of them
    // means shaders. Treat a still-loading instance as modded — mods are the
    // common case, this is a one-shot navigation decision that no later
    // update can walk back, and a modded instance sent to the shaders
    // catalogue finds nothing there.
    if (!routeData.instanceDetails.data) return "mods"
    return hasModloaders() ? "mods" : "shaders"
  }

  const columns = createAddonColumns({
    selectedCount: () => getSelectedRows().length,
    totalRows: () => addonData.filteredAddons().length,
    onSelectAll: () => {
      if (!tableInstance) return
      const totalRows = addonData.filteredAddons().length
      const selectedCount = getSelectedRows().length
      if (selectedCount === totalRows && totalRows > 0) {
        tableInstance.toggleAllRowsSelected(false)
      } else {
        tableInstance.toggleAllRowsSelected(true)
      }
    },
    onToggleMod: addonMutations.handleToggleMod,
    onDeleteMod: addonMutations.handleDeleteMod,
    getDisplayName: (row) => row.metadata?.name || row.filename,
    getSubtitle: (row) => (row.metadata?.name ? row.filename : null),
    getAddonType: (row) => row.addon_type,
    isLocked: isInstanceLocked,
    getImageUrl: (row) => {
      if (row.curseforge?.has_image) {
        return getModImageUrl(instanceId().toString(), row.id, "curseforge")
      } else if (row.modrinth?.has_image) {
        return getModImageUrl(instanceId().toString(), row.id, "modrinth")
      } else if (row.metadata?.has_image) {
        return getModImageUrl(instanceId().toString(), row.id, "metadata")
      }
      return null
    },
    isDuplicate: (row) => !!row.is_duplicate,
    getPlatformInfo: (row) => ({
      hasCurseforge: !!row.curseforge,
      hasModrinth: !!row.modrinth
    }),
    hasUpdate: (row) => !!row.has_update,
    onUpdateMod: addonMutations.handleUpdateMod,
    isModUpdating: addonData.isModUpdating,
    onSwitchVersion: addonMutations.handleSwitchVersion
  })

  return (
    <AddonsPageLayout
      isLoading={addonData.allAddons.isLoading}
      filteredAddons={addonData.filteredAddons}
      filterProps={{
        searchQuery: addonData.searchQuery,
        setSearchQuery: addonData.setSearchQuery,
        enabledAddonTypes: addonData.enabledAddonTypes,
        setEnabledAddonTypes: (type, enabled) =>
          addonData.setEnabledAddonTypes(type as any, enabled),
        addonTypes: visibleAddonTypes,
        onAddAddons: () => addonMutations.gotoSearchPage(defaultSearchType()),
        onOpenFolder: addonMutations.handleOpenFolder,
        searchInputClass: "hidden lg:flex flex-1 min-w-0",
        addButtonDisabled: isInstanceLocked(),
        addButtonTooltip: (
          <Trans key="instances:_trn_locked_cannot_apply_changes" />
        ),
        extraActions: (
          <>
            <Select
              value={addonData.platformFilter()}
              onChange={(value) =>
                value && addonData.setPlatformFilter(value as any)
              }
              options={["all", "curseforge", "modrinth", "local"]}
              placeholder=""
              disallowEmptySelection={true}
              selectionBehavior="replace"
              itemComponent={(itemProps) => {
                const getLabel = (value: string) => {
                  switch (value) {
                    case "all":
                      return t("content:_trn_filter.all")
                    case "curseforge":
                      return t("enums:_trn_curseforge")
                    case "modrinth":
                      return t("enums:_trn_modrinth")
                    case "local":
                      return t("content:_trn_filter.local")
                    default:
                      return value
                  }
                }
                const getIcon = (value: string) => {
                  switch (value) {
                    case "all":
                      return <div class="i-hugeicons:globe h-4 w-4" />
                    case "curseforge":
                      return <div class="i-simple-icons:curseforge h-4 w-4" />
                    case "modrinth":
                      return <div class="i-simple-icons:modrinth h-4 w-4" />
                    case "local":
                      return <div class="i-hugeicons:folder-01 h-4 w-4" />
                    default:
                      return null
                  }
                }
                return (
                  <SelectItem item={itemProps.item}>
                    <div class="flex items-center gap-2">
                      {getIcon(itemProps.item.rawValue)}
                      {getLabel(itemProps.item.rawValue)}
                    </div>
                  </SelectItem>
                )
              }}
            >
              <SelectTrigger class="w-32 md:w-40">
                <SelectValue<string>>
                  {(state) => {
                    const getLabel = (value: string) => {
                      switch (value) {
                        case "all":
                          return t("content:_trn_filter.all")
                        case "curseforge":
                          return t("enums:_trn_curseforge")
                        case "modrinth":
                          return t("enums:_trn_modrinth")
                        case "local":
                          return t("content:_trn_filter.local")
                        default:
                          return value
                      }
                    }
                    const getIcon = (value: string) => {
                      switch (value) {
                        case "all":
                          return <div class="i-hugeicons:globe h-4 w-4" />
                        case "curseforge":
                          return (
                            <div class="i-simple-icons:curseforge h-4 w-4" />
                          )
                        case "modrinth":
                          return <div class="i-simple-icons:modrinth h-4 w-4" />
                        case "local":
                          return <div class="i-hugeicons:folder-01 h-4 w-4" />
                        default:
                          return null
                      }
                    }
                    const selectedValue = state.selectedOption()
                    return (
                      <div class="flex items-center gap-2">
                        {getIcon(selectedValue)}
                        <span class="hidden sm:inline">
                          {getLabel(selectedValue)}
                        </span>
                      </div>
                    )
                  }}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>

            <Show when={updateCount() > 0}>
              <Tooltip>
                <TooltipTrigger>
                  <Button
                    type="secondary"
                    size="small"
                    onClick={() =>
                      addonMutations.handleUpdateAll(addonData.filteredAddons())
                    }
                    disabled={isInstanceLocked()}
                    class="px-2"
                  >
                    <div class="i-hugeicons:download-02" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <Show
                    when={!isInstanceLocked()}
                    fallback={
                      <Trans key="instances:_trn_locked_cannot_apply_changes" />
                    }
                  >
                    <Trans
                      key="content:_trn_update_all_count"
                      options={{ count: updateCount() }}
                    />
                  </Show>
                </TooltipContent>
              </Tooltip>
            </Show>
          </>
        )
      }}
      columns={columns}
      tableState={addonData}
      onTableReady={(table) => {
        tableInstance = table
      }}
      scrollContainerId="main-container-instance-details"
      onAddAddons={() => addonMutations.gotoSearchPage(defaultSearchType())}
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
              {/* Both sweeps filter on `canToggle` as well as on `enabled`,
                  and so do the `Show`s that offer them. A world is always
                  reported enabled (the scanner hardcodes it — worlds have no
                  disabled spelling), so an unfiltered "Disable all" would
                  drag every world in a mixed selection into a rename that
                  cannot disable it and that no control can undo. */}
              <Show
                when={selectedAddons().some((a) => !a.enabled && canToggle(a))}
              >
                <ContextMenuItem
                  disabled={isInstanceLocked()}
                  onSelect={() => {
                    Promise.all(
                      selectedAddons()
                        .filter((a) => !a.enabled && canToggle(a))
                        .map((a) => addonMutations.handleToggleMod(a))
                    )
                  }}
                >
                  <div class="i-hugeicons:toggle-off mr-2" />
                  {t("content:_trn_enable_all")}
                </ContextMenuItem>
              </Show>
              <Show
                when={selectedAddons().some((a) => a.enabled && canToggle(a))}
              >
                <ContextMenuItem
                  disabled={isInstanceLocked()}
                  onSelect={() => {
                    Promise.all(
                      selectedAddons()
                        .filter((a) => a.enabled && canToggle(a))
                        .map((a) => addonMutations.handleToggleMod(a))
                    )
                  }}
                >
                  <div class="i-hugeicons:toggle-off mr-2" />
                  {t("content:_trn_disable_all")}
                </ContextMenuItem>
              </Show>
              <Show when={selectedAddons().some((a) => a.has_update)}>
                <ContextMenuItem
                  disabled={isInstanceLocked()}
                  onSelect={() =>
                    addonMutations.handleUpdateSelected(selectedAddons())
                  }
                >
                  <div class="i-hugeicons:download-02 mr-2" />
                  {t("content:_trn_update_selected")}
                </ContextMenuItem>
              </Show>
              <ContextMenuSeparator />
              <ContextMenuItem
                disabled={isInstanceLocked()}
                class="text-red-400 focus:text-red-300"
                onSelect={() =>
                  addonMutations.handleDeleteSelected(selectedAddons())
                }
              >
                <div class="i-hugeicons:delete-02 mr-2" />
                {t("content:_trn_delete_selected")}
              </ContextMenuItem>
            </>
          }
        >
          <Show when={selectedAddons()[0]}>
            {(addon) => {
              const mod = () => addon()
              const displayName = () => mod().metadata?.name || mod().filename
              return (
                <>
                  <ContextMenuItem
                    onSelect={() => {
                      window.navigator.clipboard.writeText(displayName())
                      toast.success(t("notifications:_trn_copied_to_clipboard"))
                    }}
                  >
                    <div class="i-hugeicons:clipboard mr-2" />
                    {t("content:_trn_copy_name")}
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                  <Show when={canToggle(mod())}>
                    <ContextMenuItem
                      data-testid="addon-context-toggle"
                      disabled={isInstanceLocked()}
                      onSelect={() => addonMutations.handleToggleMod(mod())}
                    >
                      <div class="i-hugeicons:toggle-off mr-2" />
                      {mod().enabled
                        ? t("content:_trn_disable_mod")
                        : t("content:_trn_enable_mod")}
                    </ContextMenuItem>
                  </Show>
                  <Show when={mod().has_update}>
                    <ContextMenuItem
                      disabled={isInstanceLocked()}
                      onSelect={() => addonMutations.handleUpdateMod(mod())}
                    >
                      <div class="i-hugeicons:download-02 mr-2" />
                      {t("content:_trn_update_mod")}
                    </ContextMenuItem>
                  </Show>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    onSelect={() => {
                      modalsContext?.openModal(
                        { name: "modDetails" },
                        {
                          mod: mod(),
                          instanceId: instanceId()
                        }
                      )
                    }}
                  >
                    <div class="i-hugeicons:information-circle mr-2" />
                    {t("content:_trn_view_details")}
                  </ContextMenuItem>
                  <ContextMenuItem
                    onSelect={() => addonMutations.handleOpenFolder()}
                  >
                    <div class="i-hugeicons:folder-open mr-2" />
                    {t("instances:_trn_open_folder")}
                  </ContextMenuItem>
                  <Show when={mod().curseforge || mod().modrinth}>
                    <Show
                      when={mod().curseforge && mod().modrinth}
                      fallback={
                        <ContextMenuSub gutter={8} shift={-5}>
                          <ContextMenuSubTrigger class="data-[state=open]:bg-darkSlate-700">
                            <div class="flex items-center gap-2">
                              <Show
                                when={mod().curseforge}
                                fallback={
                                  <img
                                    src={ModrinthLogo}
                                    class="h-4 w-4"
                                    alt="Modrinth"
                                  />
                                }
                              >
                                <img
                                  src={CurseforgeLogo}
                                  class="h-4 w-4"
                                  alt="CurseForge"
                                />
                              </Show>
                              <span>
                                {t(
                                  getViewOnKey(
                                    mod().curseforge ? "curseforge" : "modrinth"
                                  )
                                )}
                              </span>
                            </div>
                          </ContextMenuSubTrigger>
                          <ContextMenuPortal>
                            <ContextMenuSubContent>
                              <ContextMenuItem
                                onSelect={() => {
                                  if (mod().curseforge) {
                                    navigator.navigate(
                                      `/addon/${mod().curseforge!.project_id}/curseforge`
                                    )
                                  } else if (mod().modrinth) {
                                    navigator.navigate(
                                      `/addon/${mod().modrinth!.project_id}/modrinth`
                                    )
                                  }
                                }}
                              >
                                <div class="i-hugeicons:dashboard-square-01 mr-2" />
                                {t("content:_trn_open_in_app")}
                              </ContextMenuItem>
                              <ContextMenuItem
                                onSelect={() => {
                                  if (mod().curseforge) {
                                    window.open(
                                      `https://www.curseforge.com/minecraft/mc-mods/${mod().curseforge!.urlslug}`,
                                      "_blank"
                                    )
                                  } else if (mod().modrinth) {
                                    window.open(
                                      `https://modrinth.com/mod/${mod().modrinth!.project_id}`,
                                      "_blank"
                                    )
                                  }
                                }}
                              >
                                <div class="flex flex-1 items-center justify-between gap-2">
                                  <div class="flex items-center gap-2">
                                    <div class="i-hugeicons:dashboard-square-01 h-4 w-4" />
                                    <span>
                                      {t("content:_trn_open_in_browser")}
                                    </span>
                                  </div>
                                  <div class="i-hugeicons:link-square-02 text-lightSlate-500 h-4 w-4" />
                                </div>
                              </ContextMenuItem>
                            </ContextMenuSubContent>
                          </ContextMenuPortal>
                        </ContextMenuSub>
                      }
                    >
                      {/* Both platforms */}
                      <ContextMenuSub gutter={8} shift={-5}>
                        <ContextMenuSubTrigger class="data-[state=open]:bg-darkSlate-700">
                          <div class="flex items-center gap-2">
                            <div class="i-hugeicons:link-square-02 h-4 w-4" />
                            <span>{t("content:_trn_view_on_platform")}</span>
                          </div>
                        </ContextMenuSubTrigger>
                        <ContextMenuPortal>
                          <ContextMenuSubContent>
                            <ContextMenuItem
                              onSelect={() =>
                                navigator.navigate(
                                  `/addon/${mod().curseforge!.project_id}/curseforge`
                                )
                              }
                            >
                              <div class="flex items-center gap-2">
                                <img
                                  src={CurseforgeLogo}
                                  class="h-4 w-4"
                                  alt="CurseForge"
                                />
                                {t("content:_trn_curseforge_open_in_app")}
                              </div>
                            </ContextMenuItem>
                            <ContextMenuItem
                              onSelect={() =>
                                window.open(
                                  `https://www.curseforge.com/minecraft/mc-mods/${mod().curseforge!.urlslug}`,
                                  "_blank"
                                )
                              }
                            >
                              <div class="flex flex-1 items-center justify-between gap-2">
                                <div class="flex items-center gap-2">
                                  <img
                                    src={CurseforgeLogo}
                                    class="h-4 w-4"
                                    alt="CurseForge"
                                  />
                                  <span>
                                    {t(
                                      "content:_trn_curseforge_open_in_browser"
                                    )}
                                  </span>
                                </div>
                                <div class="i-hugeicons:link-square-02 text-lightSlate-500 h-4 w-4" />
                              </div>
                            </ContextMenuItem>
                            <ContextMenuSeparator />
                            <ContextMenuItem
                              onSelect={() =>
                                navigator.navigate(
                                  `/addon/${mod().modrinth!.project_id}/modrinth`
                                )
                              }
                            >
                              <div class="flex items-center gap-2">
                                <img
                                  src={ModrinthLogo}
                                  class="h-4 w-4"
                                  alt="Modrinth"
                                />
                                {t("content:_trn_modrinth_open_in_app")}
                              </div>
                            </ContextMenuItem>
                            <ContextMenuItem
                              onSelect={() =>
                                window.open(
                                  `https://modrinth.com/mod/${mod().modrinth!.project_id}`,
                                  "_blank"
                                )
                              }
                            >
                              <div class="flex flex-1 items-center justify-between gap-2">
                                <div class="flex items-center gap-2">
                                  <img
                                    src={ModrinthLogo}
                                    class="h-4 w-4"
                                    alt="Modrinth"
                                  />
                                  <span>
                                    {t("content:_trn_modrinth_open_in_browser")}
                                  </span>
                                </div>
                                <div class="i-hugeicons:link-square-02 text-lightSlate-500 h-4 w-4" />
                              </div>
                            </ContextMenuItem>
                          </ContextMenuSubContent>
                        </ContextMenuPortal>
                      </ContextMenuSub>
                    </Show>
                  </Show>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    data-testid="addon-context-delete"
                    disabled={isInstanceLocked()}
                    class="text-red-400 focus:text-red-300"
                    onSelect={() => addonMutations.handleDeleteMod(mod())}
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

export default Addons
