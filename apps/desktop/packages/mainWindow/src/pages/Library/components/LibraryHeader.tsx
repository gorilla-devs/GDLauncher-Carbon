/**
 * LibraryHeader Component
 *
 * Search, filter, sort controls for the Library view.
 * Handles both folders mode and accordion mode UI states.
 * Mode-aware: uses instance or server settings based on libraryMode.
 */

import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
  Input,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsIndicator
} from "@gd/ui"
import { For, Match, Show, Switch } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import { LibraryHeaderProps, LibraryMode } from "../types"
import { InstancesGroupBy, InstancesSortBy } from "@gd/core_module/bindings"
import FeatureStatusBadge from "@/components/FeatureStatusBadge"

export function LibraryHeader(props: LibraryHeaderProps) {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()
  const modals = useModal()
  let inputRef: HTMLInputElement | undefined

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const arrangeLibraryMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.arrangeLibrary"]
  }))

  const isServerMode = () => props.libraryMode() === "servers"

  // Mode-aware settings accessors
  const currentTileSize = () =>
    isServerMode()
      ? globalStore.settings.data?.serversTileSize
      : globalStore.settings.data?.instancesTileSize

  const currentGroupBy = () =>
    isServerMode()
      ? globalStore.settings.data?.serversGroupBy
      : globalStore.settings.data?.instancesGroupBy

  const currentGroupByAsc = () =>
    isServerMode()
      ? globalStore.settings.data?.serversGroupByAsc
      : globalStore.settings.data?.instancesGroupByAsc

  const currentSortBy = () =>
    isServerMode()
      ? globalStore.settings.data?.serversSortBy
      : globalStore.settings.data?.instancesSortBy

  const currentSortByAsc = () =>
    isServerMode()
      ? globalStore.settings.data?.serversSortByAsc
      : globalStore.settings.data?.instancesSortByAsc

  // Sort by options for accordion mode
  const instanceSortByOptions: { key: InstancesSortBy; label: string }[] = [
    { key: "name", label: t("ui:_trn_name") },
    { key: "mostPlayed", label: t("ui:_trn_most_played") },
    { key: "lastPlayed", label: t("ui:_trn_last_played") },
    { key: "lastUpdated", label: t("ui:_trn_last_updated") },
    { key: "gameVersion", label: t("ui:_trn_game_version") },
    { key: "created", label: t("ui:_trn_created") }
  ]

  const serverSortByOptions: { key: string; label: string }[] = [
    { key: "name", label: t("ui:_trn_name") },
    { key: "gameVersion", label: t("ui:_trn_game_version") },
    { key: "created", label: t("ui:_trn_created") }
  ]

  const sortByOptions = () => isServerMode() ? serverSortByOptions : instanceSortByOptions

  // Group by options
  const instanceGroupByOptions: { key: InstancesGroupBy | null; label: string }[] = [
    { key: null, label: t("ui:_trn_folders") },
    { key: "gameVersion", label: t("ui:_trn_game_version") },
    { key: "modloader", label: t("ui:_trn_modloader") },
    { key: "modplatform", label: t("content:_trn_modplatform") }
  ]

  const serverGroupByOptions: { key: string | null; label: string }[] = [
    { key: null, label: t("ui:_trn_folders") },
    { key: "gameVersion", label: t("ui:_trn_game_version") }
  ]

  const groupByOptions = () => isServerMode() ? serverGroupByOptions : instanceGroupByOptions

  const isFoldersView = () => props.viewMode() === "folders"

  // Mode-aware setting helpers
  const setTileSizeSetting = (size: number) => {
    if (isServerMode()) {
      settingsMutation.mutate({ serversTileSize: { Set: size } })
    } else {
      settingsMutation.mutate({ instancesTileSize: { Set: size } })
    }
  }

  const setSortBySetting = (key: string, asc: boolean) => {
    if (isServerMode()) {
      settingsMutation.mutate({
        serversSortBy: { Set: key },
        serversSortByAsc: { Set: asc }
      })
    } else {
      settingsMutation.mutate({
        instancesSortBy: { Set: key },
        instancesSortByAsc: { Set: asc }
      })
    }
  }

  const setSortByAsc = (asc: boolean) => {
    if (isServerMode()) {
      settingsMutation.mutate({ serversSortByAsc: { Set: asc } })
    } else {
      settingsMutation.mutate({ instancesSortByAsc: { Set: asc } })
    }
  }

  const setGroupBySetting = (key: string | null) => {
    if (isServerMode()) {
      if (key === null) {
        settingsMutation.mutate({
          serversGroupBy: { Set: null },
          serversSortBy: { Set: null },
          serversGroupByAsc: { Set: true }
        })
      } else {
        settingsMutation.mutate({
          serversGroupBy: { Set: key },
          serversSortBy: { Set: "name" },
          serversGroupByAsc: { Set: true }
        })
      }
    } else {
      if (key === null) {
        settingsMutation.mutate({
          instancesGroupBy: { Set: null },
          instancesSortBy: { Set: null },
          instancesGroupByAsc: { Set: true }
        })
      } else {
        settingsMutation.mutate({
          instancesGroupBy: { Set: key },
          instancesSortBy: { Set: "name" },
          instancesGroupByAsc: { Set: true }
        })
      }
    }
  }

  const setGroupByAsc = (asc: boolean) => {
    if (isServerMode()) {
      settingsMutation.mutate({ serversGroupByAsc: { Set: asc } })
    } else {
      settingsMutation.mutate({ instancesGroupByAsc: { Set: asc } })
    }
  }

  const resetFilters = () => {
    if (isServerMode()) {
      settingsMutation.mutate({
        serversTileSize: { Set: 2 },
        serversSortBy: { Set: null },
        serversSortByAsc: { Set: false },
        serversGroupBy: { Set: null },
        serversGroupByAsc: { Set: true }
      })
    } else {
      settingsMutation.mutate({
        instancesTileSize: { Set: 2 },
        instancesSortBy: { Set: null },
        instancesSortByAsc: { Set: false },
        instancesGroupBy: { Set: null },
        instancesGroupByAsc: { Set: true }
      })
    }
    props.setTileSize(2)
  }

  return (
    <div class="bg-darkSlate-800 z-5 sticky top-0 flex items-center gap-4 py-4">
      {/* Library Mode Toggle */}
      <Tabs value={props.libraryMode()} onChange={(v) => props.setLibraryMode(v as LibraryMode)} class="h-auto w-auto flex-shrink-0">
        <TabsList class="p-0.5" size="small">
          <TabsIndicator />
          <TabsTrigger value="instances">
            <div class="flex items-center gap-1.5">
              <div class="i-hugeicons:computer h-3.5 w-3.5" />
              <Trans key="instances:_trn_instances" />
            </div>
          </TabsTrigger>
          <TabsTrigger value="servers">
            <div class="relative flex items-center gap-1.5">
              <div class="i-hugeicons:server-stack-01 h-3.5 w-3.5" />
              Servers
              <div class="absolute -top-5 -right-4">
                <FeatureStatusBadge type="beta" />
              </div>
            </div>
          </TabsTrigger>
        </TabsList>
      </Tabs>

      <Input
        ref={inputRef}
        placeholder={isServerMode() ? t("search:_trn_search_servers") : t("search:_trn_search_instances")}
        value={props.filter()}
        class="w-full rounded-full"
        onInput={(e) => props.setFilter(e.target.value)}
        icon={
          <Switch>
            <Match when={props.filter()}>
              <div
                class="hover:bg-white i-hugeicons:cancel-01"
                onClick={() => props.setFilter("")}
              />
            </Match>
            <Match when={!props.filter()}>
              <div class="i-hugeicons:search-01" />
            </Match>
          </Switch>
        }
      />

      {/* Filter dropdown */}
      <DropdownMenu>
        <DropdownMenuTrigger>
          <Button type="secondary" size="small">
            <div class="i-hugeicons:filter h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent class="w-64">
          <DropdownMenuLabel>
            <div class="flex items-center justify-between gap-2">
              <div>
                <Trans key="content:_trn_platform" />
              </div>
              <div
                class="text-lightSlate-900 hover:text-lightSlate-50 text-xs transition-colors duration-200 ease-[cubic-bezier(.4,0,.2,1)] cursor-pointer"
                onClick={resetFilters}
              >
                <Trans key="instances:_trn_reset_filters" />
              </div>
            </div>
          </DropdownMenuLabel>
          <DropdownMenuSeparator />

          <div class="flex w-full flex-col">
            {/* Tile Size */}
            <DropdownMenuSub>
              <DropdownMenuSubTrigger class="w-full">
                <div class="flex w-full items-center justify-between">
                  <Trans key="instances:_trn_instance_tile_size" />
                  <div class="flex items-center gap-2">
                    <span>{props.tileSize()}</span>
                  </div>
                </div>
              </DropdownMenuSubTrigger>
              <DropdownMenuPortal>
                <DropdownMenuSubContent>
                  <DropdownMenuLabel>
                    <Trans key="ui:_trn_tile_size" />
                  </DropdownMenuLabel>
                  <DropdownMenuRadioGroup value={props.tileSize().toString()}>
                    <For each={[1, 2, 3, 4, 5]}>
                      {(size) => (
                        <DropdownMenuRadioItem
                          value={size.toString()}
                          onSelect={() => {
                            props.setTileSize(size)
                            setTileSizeSetting(size)
                          }}
                        >
                          {size}
                        </DropdownMenuRadioItem>
                      )}
                    </For>
                  </DropdownMenuRadioGroup>
                </DropdownMenuSubContent>
              </DropdownMenuPortal>
            </DropdownMenuSub>

            {/* Sort By - only available in accordion mode */}
            <DropdownMenuSub>
              <DropdownMenuSubTrigger class="w-full" disabled={isFoldersView()}>
                <div class="flex w-full items-center justify-between">
                  <Trans key="search:_trn_sort_by" />
                  <div class="flex items-center gap-2">
                    <span>
                      {isFoldersView()
                        ? t("ui:_trn_manual")
                        : sortByOptions().find(
                            (opt) => opt.key === currentSortBy()
                          )?.label || t("ui:_trn_name")}
                    </span>
                    {!isFoldersView() && currentSortBy() && (
                      <div
                        class={`ml-2 h-4 w-4 ${
                          currentSortByAsc()
                            ? "i-hugeicons:arrange-by-letters-a-z"
                            : "i-hugeicons:arrange-by-letters-z-a"
                        }`}
                      />
                    )}
                  </div>
                </div>
              </DropdownMenuSubTrigger>
              <DropdownMenuPortal>
                <DropdownMenuSubContent>
                  <DropdownMenuLabel>
                    <Trans key="ui:_trn_sort_options" />
                  </DropdownMenuLabel>
                  <DropdownMenuRadioGroup value={currentSortBy() || ""}>
                    <For each={sortByOptions()}>
                      {(option) => (
                        <DropdownMenuRadioItem
                          value={option.key}
                          onSelect={() => {
                            if (currentSortBy() === option.key) {
                              setSortByAsc(!currentSortByAsc())
                            } else {
                              setSortBySetting(option.key, true)
                            }
                          }}
                        >
                          <div class="flex w-full items-center justify-between">
                            <span>{option.label}</span>
                            {currentSortBy() === option.key && (
                              <div
                                class={`ml-4 h-4 w-4 ${
                                  currentSortByAsc()
                                    ? "i-hugeicons:arrange-by-letters-a-z"
                                    : "i-hugeicons:arrange-by-letters-z-a"
                                }`}
                              />
                            )}
                          </div>
                        </DropdownMenuRadioItem>
                      )}
                    </For>
                  </DropdownMenuRadioGroup>
                </DropdownMenuSubContent>
              </DropdownMenuPortal>
            </DropdownMenuSub>

            {/* Group By */}
            <DropdownMenuSub>
              <DropdownMenuSubTrigger class="w-full">
                <div class="flex w-full items-center justify-between">
                  <Trans key="search:_trn_group_by" />
                  <div class="flex items-center gap-2">
                    <span>
                      {groupByOptions().find(
                        (opt) => opt.key === (currentGroupBy() ?? null)
                      )?.label || t("ui:_trn_folders")}
                    </span>
                    {!isFoldersView() && (
                      <div
                        class={`ml-2 h-4 w-4 ${
                          currentGroupByAsc()
                            ? "i-hugeicons:arrange-by-letters-a-z"
                            : "i-hugeicons:arrange-by-letters-z-a"
                        }`}
                      />
                    )}
                  </div>
                </div>
              </DropdownMenuSubTrigger>
              <DropdownMenuPortal>
                <DropdownMenuSubContent>
                  <DropdownMenuLabel>
                    <Trans key="ui:_trn_group_options" />
                  </DropdownMenuLabel>
                  <DropdownMenuRadioGroup
                    value={currentGroupBy() ?? "__folders__"}
                  >
                    <For each={groupByOptions()}>
                      {(option) => (
                        <DropdownMenuRadioItem
                          value={option.key ?? "__folders__"}
                          onSelect={() => {
                            const current = currentGroupBy() ?? null
                            if (current === option.key) {
                              if (option.key !== null) {
                                setGroupByAsc(!currentGroupByAsc())
                              }
                            } else {
                              setGroupBySetting(option.key)
                            }
                          }}
                        >
                          <div class="flex w-full items-center justify-between">
                            <span>{option.label}</span>
                            {(currentGroupBy() ?? null) === option.key &&
                              option.key !== null && (
                                <div
                                  class={`ml-4 h-4 w-4 ${
                                    currentGroupByAsc()
                                      ? "i-hugeicons:arrange-by-letters-a-z"
                                      : "i-hugeicons:arrange-by-letters-z-a"
                                  }`}
                                />
                              )}
                          </div>
                        </DropdownMenuRadioItem>
                      )}
                    </For>
                  </DropdownMenuRadioGroup>
                </DropdownMenuSubContent>
              </DropdownMenuPortal>
            </DropdownMenuSub>
          </div>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Rearrange button - only in folders mode */}
      <Show when={isFoldersView()}>
        <DropdownMenu>
          <DropdownMenuTrigger>
            <Button
              type="secondary"
              size="small"
              title={t("instances:_trn_rearrange")}
            >
              <div class="i-hugeicons:arrow-up-down h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel>
              <Trans key="instances:_trn_rearrange" />
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={() => arrangeLibraryMutation.mutate("name")}
            >
              <div class="flex items-center gap-2">
                <div class="i-hugeicons:text h-4 w-4" />
                <Trans key="ui:_trn_by_name" />
              </div>
            </DropdownMenuItem>
            <Show when={!isServerMode()}>
              <DropdownMenuItem
                onClick={() => arrangeLibraryMutation.mutate("lastPlayed")}
              >
                <div class="flex items-center gap-2">
                  <div class="i-hugeicons:clock-01 h-4 w-4" />
                  <Trans key="ui:_trn_by_last_played" />
                </div>
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => arrangeLibraryMutation.mutate("mostPlayed")}
              >
                <div class="flex items-center gap-2">
                  <div class="i-hugeicons:time-02 h-4 w-4" />
                  <Trans key="ui:_trn_by_most_played" />
                </div>
              </DropdownMenuItem>
            </Show>
            <DropdownMenuItem
              onClick={() => arrangeLibraryMutation.mutate("dateCreated")}
            >
              <div class="flex items-center gap-2">
                <div class="i-hugeicons:calendar-add-01 h-4 w-4" />
                <Trans key="ui:_trn_by_date_created" />
              </div>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </Show>

      {/* Add new instance/server button */}
      <Button
        type="primary"
        size="small"
        class="shrink-0"
        onClick={() => {
          if (isServerMode()) {
            modals?.openModal({ name: "serverCreation" })
          } else {
            modals?.openModal({ name: "instanceCreation" })
          }
        }}
      >
        <div class="i-hugeicons:add-01 h-4 w-4 shrink-0" />
        <span class="whitespace-nowrap hidden xl:inline">
          {isServerMode()
            ? t("instances:_trn_server_create_title")
            : t("library:_trn_create_new_instance")}
        </span>
      </Button>
    </div>
  )
}
