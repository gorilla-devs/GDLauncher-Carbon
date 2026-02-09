/**
 * LibraryHeader Component
 *
 * Search, filter, sort controls for the Library view.
 * Handles both folders mode and accordion mode UI states.
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
  Input
} from "@gd/ui"
import { For, Match, Show, Switch } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import FavoritesDropZone from "@/components/Library/FavoritesDropZone"
import { LibraryHeaderProps } from "../types"
import { InstancesGroupBy, InstancesSortBy } from "@gd/core_module/bindings"

export function LibraryHeader(props: LibraryHeaderProps) {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()
  const modals = useModal()
  let inputRef: HTMLInputElement | undefined
  let favoritesDropZoneRef: HTMLDivElement | undefined

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const arrangeLibraryMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.arrangeLibrary"]
  }))

  // Sort by options for accordion mode
  const sortByOptions: { key: InstancesSortBy; label: string }[] = [
    { key: "name", label: t("ui:_trn_name") },
    { key: "mostPlayed", label: t("ui:_trn_most_played") },
    { key: "lastPlayed", label: t("ui:_trn_last_played") },
    { key: "lastUpdated", label: t("ui:_trn_last_updated") },
    { key: "gameVersion", label: t("ui:_trn_game_version") },
    { key: "created", label: t("ui:_trn_created") }
  ]

  // Group by options
  const groupByOptions: { key: InstancesGroupBy | null; label: string }[] = [
    { key: null, label: t("ui:_trn_folders") },
    { key: "gameVersion", label: t("ui:_trn_game_version") },
    { key: "modloader", label: t("ui:_trn_modloader") },
    { key: "modplatform", label: t("content:_trn_modplatform") }
  ]

  const isFoldersView = () => props.viewMode() === "folders"

  return (
    <div class="bg-darkSlate-800 z-5 sticky top-0 flex items-center gap-4 py-4">
      <Show
        when={props.isFavoritesDropVisible}
        fallback={
          <Input
            ref={inputRef}
            placeholder={t("search:_trn_search_instances")}
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
        }
      >
        <div ref={favoritesDropZoneRef} class="w-full h-10">
          <FavoritesDropZone
            instances={globalStore.instances.data || []}
            containerRef={favoritesDropZoneRef}
          />
        </div>
      </Show>

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
                onClick={() => {
                  settingsMutation.mutate({
                    instancesTileSize: { Set: 2 },
                    instancesSortBy: { Set: null },
                    instancesSortByAsc: { Set: false },
                    instancesGroupBy: { Set: null },
                    instancesGroupByAsc: { Set: true }
                  })
                  props.setTileSize(2)
                }}
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
                            settingsMutation.mutate({
                              instancesTileSize: { Set: size }
                            })
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
                        : sortByOptions.find(
                            (opt) =>
                              opt.key ===
                              globalStore.settings.data?.instancesSortBy
                          )?.label || t("ui:_trn_name")}
                    </span>
                    {!isFoldersView() &&
                      globalStore.settings.data?.instancesSortBy && (
                        <div
                          class={`ml-2 h-4 w-4 ${
                            globalStore.settings.data?.instancesSortByAsc
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
                  <DropdownMenuRadioGroup
                    value={globalStore.settings.data?.instancesSortBy || ""}
                  >
                    <For each={sortByOptions}>
                      {(option) => (
                        <DropdownMenuRadioItem
                          value={option.key}
                          onSelect={() => {
                            const currentOption =
                              globalStore.settings.data?.instancesSortBy
                            const currentDirection =
                              globalStore.settings.data?.instancesSortByAsc

                            if (currentOption === option.key) {
                              settingsMutation.mutate({
                                instancesSortByAsc: { Set: !currentDirection }
                              })
                            } else {
                              settingsMutation.mutate({
                                instancesSortBy: { Set: option.key },
                                instancesSortByAsc: { Set: true }
                              })
                            }
                          }}
                        >
                          <div class="flex w-full items-center justify-between">
                            <span>{option.label}</span>
                            {globalStore.settings.data?.instancesSortBy ===
                              option.key && (
                              <div
                                class={`ml-4 h-4 w-4 ${
                                  globalStore.settings.data?.instancesSortByAsc
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
                      {groupByOptions.find(
                        (opt) =>
                          opt.key ===
                          (globalStore.settings.data?.instancesGroupBy ?? null)
                      )?.label || t("ui:_trn_folders")}
                    </span>
                    {!isFoldersView() && (
                      <div
                        class={`ml-2 h-4 w-4 ${
                          globalStore.settings.data?.instancesGroupByAsc
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
                    value={
                      globalStore.settings.data?.instancesGroupBy ?? "__folders__"
                    }
                  >
                    <For each={groupByOptions}>
                      {(option) => (
                        <DropdownMenuRadioItem
                          value={option.key ?? "__folders__"}
                          onSelect={() => {
                            const currentOption =
                              globalStore.settings.data?.instancesGroupBy ?? null
                            const currentDirection =
                              globalStore.settings.data?.instancesGroupByAsc

                            if (currentOption === option.key) {
                              if (option.key !== null) {
                                settingsMutation.mutate({
                                  instancesGroupByAsc: { Set: !currentDirection }
                                })
                              }
                            } else {
                              if (option.key === null) {
                                settingsMutation.mutate({
                                  instancesGroupBy: { Set: null },
                                  instancesSortBy: { Set: null },
                                  instancesGroupByAsc: { Set: true }
                                })
                              } else {
                                settingsMutation.mutate({
                                  instancesGroupBy: { Set: option.key },
                                  instancesSortBy: { Set: "name" },
                                  instancesGroupByAsc: { Set: true }
                                })
                              }
                            }
                          }}
                        >
                          <div class="flex w-full items-center justify-between">
                            <span>{option.label}</span>
                            {(globalStore.settings.data?.instancesGroupBy ??
                              null) === option.key &&
                              option.key !== null && (
                                <div
                                  class={`ml-4 h-4 w-4 ${
                                    globalStore.settings.data?.instancesGroupByAsc
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

      {/* Rearrange and Create Folder buttons - only in folders mode */}
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
    </div>
  )
}
