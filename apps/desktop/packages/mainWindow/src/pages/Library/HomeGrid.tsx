import {
  Button,
  Collapsable,
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
  DropdownMenu,
  DropdownMenuContent,
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
  Skeleton
} from "@gd/ui"
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  onMount
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import InstanceTile from "@/components/InstanceTile"
import UnstableCard from "@/components/UnstableCard"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import {
  InstancesGroupBy,
  InstancesSortBy,
  ListInstance,
  ValidListInstance
} from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { createStore, reconcile } from "solid-js/store"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"

let initAnimationRan = false

const HomeGrid = () => {
  const [t] = useTransContext()

  const [filter, setFilter] = createSignal("")

  const globalStore = useGlobalStore()

  const modals = useModal()

  const [instancesTileSize, setInstancesTileSize] = createSignal(2)

  const [instances, setInstances] = createStore<
    {
      id: string | number | null
      name: string
      instances: ListInstance[]
    }[]
  >([])

  createEffect(() => {
    setInstancesTileSize(globalStore.settings.data?.instancesTileSize!)
  })

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  let inputRef: HTMLInputElement | undefined

  type Groups = Record<
    string | number,
    {
      id: string | number | null
      name: string
      instances: ListInstance[]
    }
  >

  const filteredGroups = createMemo(() => {
    const _groups: Groups = {}

    const nameFilter = filter().replaceAll(" ", "").toLowerCase()

    if (globalStore.settings.data?.instancesGroupBy === "group") {
      _groups.favorites = {
        id: -1,
        name: t("instances:_trn_favorites"),
        instances: []
      }
    }

    for (const instance of globalStore.instances.data || []) {
      let groupId = null
      let groupName = null

      const validInstance =
        instance.status.status === "valid" ? instance.status.value : undefined

      if (globalStore.settings.data?.instancesGroupBy === "group") {
        const _groupName = globalStore.instanceGroups.data?.find(
          (group) => group.id === instance.group_id
        )?.name

        groupName =
          _groupName === "localize➽default"
            ? t("general:_trn_default")
            : _groupName
        groupId = instance.group_id
      } else if (
        globalStore.settings.data?.instancesGroupBy === "gameVersion"
      ) {
        if (instance.status.status === "valid") {
          groupName = validInstance?.mc_version
        }
      } else if (globalStore.settings.data?.instancesGroupBy === "modloader") {
        if (instance.status.status === "valid") {
          groupName = validInstance?.modloader || "vanilla"
        }
      } else if (
        globalStore.settings.data?.instancesGroupBy === "modplatform"
      ) {
        if (instance.status.status === "valid") {
          groupName = validInstance?.modpack?.type
        }
      }

      if (!groupName) {
        continue
      }

      if (!_groups[groupName]) {
        _groups[groupName] = {
          id: groupId,
          name: groupName,
          instances: []
        }
      }

      if (
        instance.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      ) {
        if (
          globalStore.settings.data?.instancesGroupBy === "group" &&
          instance.favorite
        ) {
          _groups.favorites.instances.push(instance)
        }
        _groups[groupName].instances.push(instance)
      }
    }

    // sort groups
    for (const key in _groups) {
      _groups[key].instances.sort((a, b) => {
        let comparisonResult = 0 // Default comparison result

        if (globalStore.settings.data?.instancesSortBy === "name") {
          comparisonResult = a.name.localeCompare(b.name)
        } else if (
          globalStore.settings.data?.instancesSortBy === "mostPlayed"
        ) {
          comparisonResult = (a.seconds_played || 0) - (b.seconds_played || 0)
        } else if (
          globalStore.settings.data?.instancesSortBy === "lastPlayed"
        ) {
          const aLastPlayed = a.last_played ? Date.parse(a.last_played) : 0
          const bLastPlayed = b.last_played ? Date.parse(b.last_played) : 0
          comparisonResult = aLastPlayed - bLastPlayed
        } else if (
          globalStore.settings.data?.instancesSortBy === "lastUpdated"
        ) {
          const aLastUpdated = a.date_updated ? Date.parse(a.date_updated) : 0
          const bLastUpdated = b.date_updated ? Date.parse(b.date_updated) : 0
          comparisonResult = aLastUpdated - bLastUpdated
        } else if (
          globalStore.settings.data?.instancesSortBy === "gameVersion"
        ) {
          comparisonResult = (
            (a.status.value as ValidListInstance).mc_version || ""
          ).localeCompare(
            (b.status.value as ValidListInstance).mc_version || "",
            undefined,
            { numeric: true, sensitivity: "base" }
          )
        } else if (globalStore.settings.data?.instancesSortBy === "created") {
          const aCreated = a.date_created ? Date.parse(a.date_created) : 0
          const bCreated = b.date_created ? Date.parse(b.date_created) : 0
          comparisonResult = aCreated - bCreated
        }

        // If descending order is selected, invert the comparison result
        if (!globalStore.settings.data?.instancesSortByAsc) {
          comparisonResult = -comparisonResult
        }

        // Use name as a secondary sort criteria to ensure consistent order where primary criteria are equal
        return comparisonResult || a.name.localeCompare(b.name)
      })
    }

    return _groups
  })

  const iterableFilteredGroups = createMemo(() => {
    const iterable = Object.values(filteredGroups())

    if (globalStore.settings.data?.instancesGroupBy === "gameVersion") {
      iterable.sort((a, b) => {
        if (globalStore.settings.data?.instancesGroupByAsc) {
          return a.name.localeCompare(b.name, undefined, {
            numeric: true,
            sensitivity: "base"
          })
        } else {
          return b.name.localeCompare(a.name, undefined, {
            numeric: true,
            sensitivity: "base"
          })
        }
      })
    } else {
      iterable.sort((a, b) => {
        if (a.name === t("instances:_trn_favorites")) {
          return -1
        }

        if (b.name === t("instances:_trn_favorites")) {
          return 1
        }

        if (globalStore.settings.data?.instancesGroupByAsc) {
          return a.name.localeCompare(b.name)
        } else {
          return b.name.localeCompare(a.name)
        }
      })
    }

    return iterable
  })

  createEffect(() => {
    setInstances(reconcile(iterableFilteredGroups()))
  })

  const sortByOptions: {
    key: InstancesSortBy
    label: string
  }[] = [
    {
      key: "name",
      label: t("ui:_trn_name")
    },
    {
      key: "mostPlayed",
      label: t("ui:_trn_most_played")
    },
    {
      key: "lastPlayed",
      label: t("ui:_trn_last_played")
    },
    {
      key: "lastUpdated",
      label: t("ui:_trn_last_updated")
    },
    {
      key: "gameVersion",
      label: t("ui:_trn_game_version")
    },
    {
      key: "created",
      label: t("ui:_trn_created")
    }
  ]

  const groupByOptions: {
    key: InstancesGroupBy
    label: string
  }[] = [
    {
      key: "group",
      label: t("ui:_trn_group")
    },
    {
      key: "gameVersion",
      label: t("ui:_trn_game_version")
    },
    {
      key: "modloader",
      label: t("ui:_trn_modloader")
    },
    {
      key: "modplatform",
      label: t("content:_trn_modplatform")
    }
  ]

  return (
    <div class="p-6">
      <UnstableCard />
      <Switch>
        <Match when={globalStore.instances.isLoading}>
          <div>
            <Skeleton.instances />
          </div>
        </Match>
        <Match
          when={
            globalStore.instances?.data?.length === 0 &&
            !globalStore.instances.isLoading
          }
        >
          <div class="mt-12 flex h-full w-full flex-col items-center justify-center gap-6">
            <PlaceholderGorilla
              size={14}
              variant="Welcoming Gorilla - Open Arms"
            />
            <p class="text-lightSlate-700 max-w-100 text-center">
              <Trans key="instances:_trn_no_instances_text" />
            </p>
          </div>
        </Match>
        <Match
          when={
            (globalStore.instances?.data?.length || 0) > 0 &&
            !globalStore.instances.isLoading
          }
        >
          <div>
            <div class="bg-darkSlate-800 z-5 sticky top-0 flex items-center gap-4 py-4">
              <Input
                ref={inputRef}
                placeholder={t("search:_trn_search_instances")}
                value={filter()}
                class="w-full rounded-full"
                onInput={(e) => setFilter(e.target.value)}
                disabled={iterableFilteredGroups().length === 0}
                icon={
                  <Switch>
                    <Match when={filter()}>
                      <div
                        class="hover:bg-white i-hugeicons:cancel-01"
                        onClick={() => {
                          setFilter("")
                        }}
                      />
                    </Match>
                    <Match when={!filter()}>
                      <div class="i-hugeicons:search-01" />
                    </Match>
                  </Switch>
                }
              />
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
                        class="text-lightSlate-900 hover:text-lightSlate-50 text-xs transition-colors duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
                        onClick={() => {
                          // Reset all filter settings to defaults
                          settingsMutation.mutate({
                            instancesTileSize: { Set: 2 },
                            instancesSortBy: { Set: "name" },
                            instancesSortByAsc: { Set: true },
                            instancesGroupBy: { Set: "group" },
                            instancesGroupByAsc: { Set: true }
                          })
                          setInstancesTileSize(2)
                        }}
                      >
                        <Trans key="instances:_trn_reset_filters" />
                      </div>
                    </div>
                  </DropdownMenuLabel>
                  <DropdownMenuSeparator />

                  <div class="flex w-full flex-col">
                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger class="w-full">
                        <div class="flex w-full items-center justify-between">
                          <Trans key="instances:_trn_instance_tile_size" />
                          <div class="flex items-center gap-2">
                            <span>{instancesTileSize()}</span>
                          </div>
                        </div>
                      </DropdownMenuSubTrigger>
                      <DropdownMenuPortal>
                        <DropdownMenuSubContent>
                          <DropdownMenuLabel>
                            <Trans key="ui:_trn_tile_size" />
                          </DropdownMenuLabel>
                          <DropdownMenuRadioGroup
                            value={instancesTileSize().toString()}
                          >
                            <For each={[1, 2, 3, 4, 5]}>
                              {(size) => (
                                <DropdownMenuRadioItem
                                  value={size.toString()}
                                  onSelect={() => {
                                    setInstancesTileSize(size)
                                    settingsMutation.mutate({
                                      instancesTileSize: {
                                        Set: size
                                      }
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

                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger class="w-full">
                        <div class="flex w-full items-center justify-between">
                          <Trans key="search:_trn_sort_by" />
                          <div class="flex items-center gap-2">
                            <span>
                              {sortByOptions.find(
                                (opt) =>
                                  opt.key ===
                                  globalStore.settings.data?.instancesSortBy
                              )?.label || "Name"}
                            </span>
                            {globalStore.settings.data?.instancesSortBy && (
                              <div
                                class={`ml-2 h-4 w-4 ${globalStore.settings.data?.instancesSortByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
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
                            value={
                              globalStore.settings.data?.instancesSortBy || ""
                            }
                          >
                            <For each={sortByOptions}>
                              {(option) => (
                                <DropdownMenuRadioItem
                                  value={option.key}
                                  onSelect={() => {
                                    const currentOption =
                                      globalStore.settings.data?.instancesSortBy
                                    const currentDirection =
                                      globalStore.settings.data
                                        ?.instancesSortByAsc

                                    // If clicking the same option
                                    if (currentOption === option.key) {
                                      // Toggle direction
                                      settingsMutation.mutate({
                                        instancesSortByAsc: {
                                          Set: !currentDirection
                                        }
                                      })
                                    } else {
                                      // New option, set to ascending by default
                                      settingsMutation.mutate({
                                        instancesSortBy: {
                                          Set: option.key
                                        },
                                        instancesSortByAsc: {
                                          Set: true
                                        }
                                      })
                                    }
                                  }}
                                >
                                  <div class="flex w-full items-center justify-between">
                                    <span>{option.label}</span>
                                    {globalStore.settings.data
                                      ?.instancesSortBy === option.key && (
                                      <div
                                        class={`ml-4 h-4 w-4 ${globalStore.settings.data?.instancesSortByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
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

                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger class="w-full">
                        <div class="flex w-full items-center justify-between">
                          <Trans key="search:_trn_group_by" />
                          <div class="flex items-center gap-2">
                            <span>
                              {groupByOptions.find(
                                (opt) =>
                                  opt.key ===
                                  globalStore.settings.data?.instancesGroupBy
                              )?.label || "Group"}
                            </span>
                            {globalStore.settings.data?.instancesGroupBy && (
                              <div
                                class={`ml-2 h-4 w-4 ${globalStore.settings.data?.instancesGroupByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
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
                              globalStore.settings.data?.instancesGroupBy || ""
                            }
                          >
                            <For each={groupByOptions}>
                              {(option) => (
                                <DropdownMenuRadioItem
                                  value={option.key}
                                  onSelect={() => {
                                    const currentOption =
                                      globalStore.settings.data
                                        ?.instancesGroupBy
                                    const currentDirection =
                                      globalStore.settings.data
                                        ?.instancesGroupByAsc

                                    // If clicking the same option
                                    if (currentOption === option.key) {
                                      // Toggle direction
                                      settingsMutation.mutate({
                                        instancesGroupByAsc: {
                                          Set: !currentDirection
                                        }
                                      })
                                    } else {
                                      // New option, set to ascending by default
                                      settingsMutation.mutate({
                                        instancesGroupBy: {
                                          Set: option.key
                                        },
                                        instancesGroupByAsc: {
                                          Set: true
                                        }
                                      })
                                    }
                                  }}
                                >
                                  <div class="flex w-full items-center justify-between">
                                    <span>{option.label}</span>
                                    {globalStore.settings.data
                                      ?.instancesGroupBy === option.key && (
                                      <div
                                        class={`ml-4 h-4 w-4 ${globalStore.settings.data?.instancesGroupByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
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
            </div>
            <ContextMenu>
              <ContextMenuTrigger>
                <div class="mt-4">
                  <For each={instances || []}>
                    {(group, i) => {
                      return (
                        <Show when={group.instances.length > 0}>
                          <Collapsable
                            noPadding
                            title={
                              <>
                                <span>{group.name}</span>
                              </>
                            }
                            size="standard"
                          >
                            <div
                              class="mt-4 flex flex-wrap gap-x-4"
                              classList={{
                                "gap-y-4": instancesTileSize() === 1,
                                "gap-y-6": instancesTileSize() === 2,
                                "gap-y-8": instancesTileSize() === 3,
                                "gap-y-10": instancesTileSize() === 4,
                                "gap-y-12": instancesTileSize() === 5
                              }}
                            >
                              <For each={group.instances}>
                                {(instance, j) => {
                                  let ref: HTMLDivElement | undefined

                                  const instancesCountInPreviousGroups =
                                    instances
                                      .slice(0, i())
                                      .reduce(
                                        (acc, group) =>
                                          acc + group.instances.length,
                                        0
                                      )

                                  const baseDelay = 300

                                  const groupDelay =
                                    i() * 60 +
                                    60 * instancesCountInPreviousGroups

                                  const instanceDelay = j() * 60

                                  const totalDelay =
                                    baseDelay + groupDelay + instanceDelay

                                  onMount(() => {
                                    if (ref && !initAnimationRan) {
                                      ref.animate(
                                        [
                                          {
                                            opacity: 0
                                          },
                                          {
                                            opacity: 1
                                          }
                                        ],
                                        {
                                          duration: 250,
                                          delay: totalDelay,
                                          easing: "linear",
                                          fill: "forwards"
                                        }
                                      )
                                    }

                                    if (
                                      i() === instances.length - 1 &&
                                      j() === group.instances.length - 1
                                    ) {
                                      requestAnimationFrame(() => {
                                        initAnimationRan = true
                                      })
                                    }
                                  })

                                  return (
                                    <div
                                      ref={ref}
                                      classList={{
                                        "opacity-0": !initAnimationRan
                                      }}
                                    >
                                      <InstanceTile
                                        instance={instance}
                                        identifier={`${group.id?.toString() || group.name} - ${instance.id}`}
                                        size={instancesTileSize() as any}
                                      />
                                    </div>
                                  )
                                }}
                              </For>
                            </div>
                          </Collapsable>
                        </Show>
                      )
                    }}
                  </For>
                </div>
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuGroup>
                  <ContextMenuGroupLabel>
                    <Trans key="library:_trn_add_new_instance" />
                  </ContextMenuGroupLabel>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() => {
                      modals?.openModal({
                        name: "instanceCreation"
                      })
                    }}
                  >
                    <div class="i-hugeicons:file-add h-4 w-4" />
                    <Trans key="library:_trn_create_new_instance" />
                  </ContextMenuItem>
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() => {
                      modals?.openModal(
                        {
                          name: "instanceCreation"
                        },
                        {
                          import: true
                        }
                      )
                    }}
                  >
                    <div class="i-hugeicons:download-02 h-4 w-4" />
                    <Trans key="library:_trn_import_instance" />
                  </ContextMenuItem>
                </ContextMenuGroup>
              </ContextMenuContent>
            </ContextMenu>
          </div>
        </Match>
      </Switch>
    </div>
  )
}

export default HomeGrid
