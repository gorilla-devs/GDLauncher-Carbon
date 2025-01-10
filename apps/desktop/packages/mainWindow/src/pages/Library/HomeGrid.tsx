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
  Dropdown,
  Input,
  News,
  Popover,
  Skeleton,
  Slider
} from "@gd/ui"
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onMount
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import InstanceTile from "@/components/InstanceTile"
import skull from "/assets/images/icons/skull.png"
import DefaultImg from "/assets/images/default-instance-img.png"
import UnstableCard from "@/components/UnstableCard"
import FeaturedModpackTile from "./FeaturedModpackTile"
import {
  InstancesGroupBy,
  InstancesSortBy,
  ListInstance,
  ValidListInstance
} from "@gd/core_module/bindings"
import { initNews } from "@/utils/news"
import { rspc } from "@/utils/rspcClient"
import { createStore, reconcile } from "solid-js/store"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"

const NewsWrapper = () => {
  const newsInitializer = initNews()

  const [news] = createResource(() => newsInitializer)

  return (
    <div class="flex gap-4">
      <div class="flex-1 flex-grow">
        <Switch>
          <Match when={(news()?.length || 0) > 0}>
            <News
              slides={news()}
              onClick={(news) => {
                window.openExternalLink(news.url || "")
              }}
              fallBackImg={DefaultImg}
            />
          </Match>
          <Match when={news.length === 0}>
            <Skeleton.news />
          </Match>
        </Switch>
      </div>
      <FeaturedModpackTile />
    </div>
  )
}

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
    const timeStart = performance.now()

    const _groups: Groups = {}

    const nameFilter = filter().replaceAll(" ", "").toLowerCase()

    if (globalStore.settings.data?.instancesGroupBy === "group") {
      _groups.favorites = {
        id: -1,
        name: t("favorites"),
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
          _groupName === "localize➽default" ? t("default") : _groupName
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

    console.log(
      `Recomputing filtered groups ${performance.now() - timeStart} ms`
    )

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
        if (a.name === t("favorites")) {
          return -1
        }

        if (b.name === t("favorites")) {
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
      label: t("general.name")
    },
    {
      key: "mostPlayed",
      label: t("general.most_played")
    },
    {
      key: "lastPlayed",
      label: t("general.last_played")
    },
    {
      key: "lastUpdated",
      label: t("general.last_updated")
    },
    {
      key: "gameVersion",
      label: t("general.game_version")
    },
    {
      key: "created",
      label: t("general.created")
    }
  ]

  const groupByOptions: {
    key: InstancesGroupBy
    label: string
  }[] = [
    {
      key: "group",
      label: t("general.group")
    },
    {
      key: "gameVersion",
      label: t("general.game_version")
    },
    {
      key: "modloader",
      label: t("general.modloader")
    },
    {
      key: "modplatform",
      label: t("general.modplatform")
    }
  ]

  return (
    <div class="p-6">
      <UnstableCard />
      <Show when={globalStore.settings.data?.showNews}>
        <NewsWrapper />
      </Show>
      <Switch>
        <Match when={globalStore.instances.isLoading}>
          <div class="mt-8">
            <Skeleton.instances />
          </div>
        </Match>
        <Match
          when={
            globalStore.instances?.data?.length === 0 &&
            !globalStore.instances.isLoading
          }
        >
          <div class="mt-12 flex h-full w-full flex-col items-center justify-center">
            <img src={skull} class="h-16 w-16" />
            <p class="text-lightSlate-700 max-w-100 text-center">
              <Trans key="instance.no_instances_text" />
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
            <div class="bg-darkSlate-800 sticky top-0 mt-8 flex items-center gap-4 py-4 z-5">
              <Input
                ref={inputRef}
                placeholder={t("search_instances")}
                value={filter()}
                class="w-full rounded-full"
                onInput={(e) => setFilter(e.target.value)}
                disabled={iterableFilteredGroups().length === 0}
                icon={
                  <Switch>
                    <Match when={filter()}>
                      <div
                        onClick={() => {
                          setFilter("")
                        }}
                        class="i-ri:close-line hover:bg-white"
                      />
                    </Match>
                    <Match when={!filter()}>
                      <div class="i-ri:search-line" />
                    </Match>
                  </Switch>
                }
              />
              <Popover
                trigger="click"
                noTip
                noPadding
                content={() => (
                  <div class="w-100 flex h-auto flex-col gap-y-6 p-4">
                    <div class="mb-4 text-2xl">
                      <Trans key="general.instances_filters" />
                    </div>
                    <div class="flex w-full items-center justify-between">
                      <div>
                        <Trans key="general.instance_tile_size" />
                      </div>
                      <div class="w-50 flex items-center">
                        <Slider
                          min={1}
                          max={5}
                          marks={[]}
                          steps={1}
                          value={instancesTileSize()}
                          onChange={(value) => {
                            if (!value) return

                            setInstancesTileSize(value)
                          }}
                          OnRelease={(value) => {
                            if (
                              value ===
                              globalStore.settings.data?.instancesTileSize
                            ) {
                              return
                            }

                            settingsMutation.mutate({
                              instancesTileSize: {
                                Set: value
                              }
                            })
                          }}
                        />
                      </div>
                    </div>
                    <div class="flex w-full items-center justify-between">
                      <div>
                        <Trans key="general.sort_by" />
                      </div>
                      <div class="flex items-center gap-4">
                        <Dropdown
                          class="w-40"
                          options={sortByOptions}
                          icon={<div class="i-ri:price-tag-3-fill" />}
                          value={globalStore.settings.data?.instancesSortBy}
                          onChange={(val) => {
                            settingsMutation.mutate({
                              instancesSortBy: {
                                Set: val.key as InstancesSortBy
                              }
                            })
                          }}
                        />
                        <div
                          class="text-lightSlate-700 hover:text-lightSlate-50 h-6 w-6"
                          classList={{
                            "i-ri:sort-alphabet-asc":
                              globalStore.settings.data?.instancesSortByAsc,
                            "i-ri:sort-alphabet-desc":
                              !globalStore.settings.data?.instancesSortByAsc
                          }}
                          onClick={() => {
                            settingsMutation.mutate({
                              instancesSortByAsc: {
                                Set: !globalStore.settings.data
                                  ?.instancesSortByAsc
                              }
                            })
                          }}
                        />
                      </div>
                    </div>
                    <div class="flex w-full items-center justify-between">
                      <div>
                        <Trans key="general.group_by" />
                      </div>
                      <div class="flex items-center gap-4">
                        <Dropdown
                          class="w-40"
                          options={groupByOptions}
                          icon={<div class="i-ri:price-tag-3-fill" />}
                          value={globalStore.settings.data?.instancesGroupBy}
                          onChange={(val) => {
                            settingsMutation.mutate({
                              instancesGroupBy: {
                                Set: val.key as InstancesGroupBy
                              }
                            })
                          }}
                        />
                        <div
                          class="text-lightSlate-700 hover:text-lightSlate-50 h-6 w-6"
                          classList={{
                            "i-ri:sort-alphabet-asc":
                              globalStore.settings.data?.instancesGroupByAsc,
                            "i-ri:sort-alphabet-desc":
                              !globalStore.settings.data?.instancesGroupByAsc
                          }}
                          onClick={() => {
                            settingsMutation.mutate({
                              instancesGroupByAsc: {
                                Set: !globalStore.settings.data
                                  ?.instancesGroupByAsc
                              }
                            })
                          }}
                        />
                      </div>
                    </div>
                    <div class="flex justify-end">
                      <span
                        class="text-lightSlate-700 hover:text-lightSlate-50 mt-4"
                        onClick={() => {
                          settingsMutation.mutate({
                            instancesSortBy: {
                              Set: "name"
                            },
                            instancesSortByAsc: {
                              Set: true
                            },
                            instancesGroupBy: {
                              Set: "group"
                            },
                            instancesGroupByAsc: {
                              Set: true
                            },
                            instancesTileSize: {
                              Set: 2
                            }
                          })
                        }}
                      >
                        <Trans key="general.reset_filters" />
                      </span>
                    </div>
                  </div>
                )}
              >
                <Button type="secondary" size="small">
                  <i class="i-ri:filter-fill h-4 w-4" />
                </Button>
              </Popover>
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
                                {/* <img
                            class="w-6 h-6"
                            src={getCFModloaderIcon(key as CFFEModLoaderType)}
                          /> */}
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
                    Add New Instance
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
                    <div class="i-ri:file-add-fill h-4 w-4" />
                    Create New Instance
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
                    <div class="i-ri:import-fill h-4 w-4" />
                    Import Instance
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
