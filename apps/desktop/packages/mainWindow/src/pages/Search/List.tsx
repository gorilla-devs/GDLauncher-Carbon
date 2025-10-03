import { ListItem } from "./ListItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  Skeleton,
  Tabs
} from "@gd/ui"
import { Tab, TabList } from "@gd/ui"
import { For, onMount, Show, createMemo } from "solid-js"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useLocation, useParams } from "@solidjs/router"
import FiltersDisplay from "./FiltersDisplay"
import { FiltersDropdown } from "./FiltersDropdown"
import { rspc } from "@/utils/rspcClient"
import { Trans, useTransContext } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"

interface SearchTab {
  label: string
  value: FEUnifiedSearchType
  icon: string
  path: string
}

export function List() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const params = useParams()
  const location = useLocation()
  const [t] = useTransContext()
  const globalStore = useGlobalStore()

  const instanceId = () => searchContext?.selectedInstanceId() || NaN
  const instanceMods = () => searchContext?.selectedInstanceMods

  const instance = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
  }))

  const defaultFallbackType: () => FEUnifiedSearchType = () => {
    if (!instanceId()) {
      return "modpack"
    }

    if (instance?.data?.modloaders?.length ?? 0 > 0) {
      return "mod"
    }

    return "shader"
  }

  const type = () =>
    (params.type || defaultFallbackType()) as FEUnifiedSearchType

  if (type() !== searchContext?.searchQuery().projectType) {
    searchContext?.setSearchQuery((prev) => ({
      ...prev,
      projectType: type()
    }))
  }

  const shouldShowDataPackTab = createMemo(() => {
    if (!instanceId() || !searchContext?.selectedInstance?.data) {
      return null
    }

    const instanceVersion = searchContext.selectedInstance.data.version
    const instanceMCVersionPos = globalStore.minecraftVersions.data?.findIndex(
      (v) => v.id === instanceVersion
    )

    const Pos1_13 = globalStore.minecraftVersions.data?.findIndex(
      (v) => v.id === "1.13"
    )

    // Check version requirements for different addon types
    if ((instanceMCVersionPos ?? 0) > (Pos1_13 ?? 0)) {
      return false
    }

    return true
  })

  const projectTypeTabs: () => SearchTab[] = () => {
    let tabs: SearchTab[] = []

    if (!instanceId()) {
      tabs.push({
        label: t("search.modpacks"),
        value: "modpack",
        icon: "i-ri:folder-fill",
        path: "/search/modpack"
      })
    }

    if (
      !instanceId() ||
      (instanceId() && (instance.data?.modloaders?.length ?? 0) > 0)
    ) {
      tabs.push({
        label: t("search.mods"),
        value: "mod",
        icon: "i-ri:file-text-fill",
        path: "/search/mod"
      })
    }

    tabs = tabs.concat([
      {
        label: t("search.shaders"),
        value: "shader",
        icon: "i-ri:paint-fill",
        path: "/search/shader"
      },
      {
        label: t("search.resource_packs"),
        value: "resourcePack",
        icon: "i-ri:folder-fill",
        path: "/search/resourcePack"
      }
    ])

    if (shouldShowDataPackTab()) {
      tabs.push({
        label: t("search.data_packs"),
        value: "datapack",
        icon: "i-ri:folder-fill",
        path: "/search/datapack"
      })
    }

    tabs.push({
      label: t("search.worlds"),
      value: "world",
      icon: "i-ri:folder-fill",
      path: "/search/world"
    })

    return tabs
  }

  onMount(() => {
    queueMicrotask(() => {
      searchContext?.ref()?.scrollTo(searchContext.lastScrollOffset())
    })
  })

  const installedMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
  }))

  const lookupTableInstalledMods = createMemo(() => {
    const curseforgeMods =
      installedMods.data?.reduce((acc: string[], mod) => {
        if (mod.curseforge?.project_id) {
          acc.push(mod.curseforge.project_id.toString())
        }
        return acc
      }, []) || []

    const modrinthMods =
      installedMods.data?.reduce((acc: string[], mod) => {
        if (mod.modrinth?.project_id) {
          acc.push(mod.modrinth.project_id)
        }
        return acc
      }, []) || []

    const map = new Set([...curseforgeMods, ...modrinthMods])

    return map
  })

  return (
    <div class="flex h-full flex-col overflow-hidden">
      <div class="flex w-full p-6 gap-8">
        <div
          class="w-48 items-center"
          classList={{
            hidden: !instanceId(),
            flex: !!instanceId()
          }}
        >
          <Button
            size="small"
            type="outline"
            onClick={() => {
              navigator.navigate(`/library/${instanceId()}/addons`)
            }}
          >
            <div class="i-ri:arrow-left-line" />
            <Trans key="search.go_back" />
          </Button>
        </div>
        <Tabs
          defaultIndex={projectTypeTabs().findIndex(
            (tab) => tab.value === type()
          )}
        >
          <TabList aligment="between">
            <For each={projectTypeTabs()}>
              {(tab, index) => (
                <Tab
                  onClick={() => {
                    if (
                      index() ===
                      projectTypeTabs().findIndex((tab) => tab.value === type())
                    )
                      return

                    navigator.navigate(`${tab.path}${location.search}`)

                    queueMicrotask(() => {
                      searchContext?.setSearchQuery((prev) => ({
                        ...prev,
                        projectType: tab.value
                      }))
                    })
                  }}
                >
                  {tab.label}
                </Tab>
              )}
            </For>
          </TabList>
        </Tabs>
        <div class="flex items-center gap-2">
          <DropdownMenu>
            <DropdownMenuTrigger>
              <Button type="glass" size="small" class="text-xs">
                <div class="flex items-center gap-1">
                  <div class="i-ri:filter-3-line" />
                  <div>
                    <Trans key="search.filters" />
                  </div>
                  <div class="i-ri:arrow-down-s-line text-xs" />
                </div>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <FiltersDropdown />
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      <FiltersDisplay />
      <div class="flex-1 overflow-hidden">
        <Show
          when={!searchContext?.isInitialLoading()}
          fallback={<Skeleton.searchList />}
        >
          <Show
            when={(searchContext?.allRows() || []).length > 0}
            fallback={
              <div class="flex flex-col items-center justify-center px-6 py-16 text-center">
                <div class="i-ri:search-line mb-4 text-6xl text-gray-400" />
                <h3 class="mb-2 text-xl font-semibold text-gray-300">
                  <Trans key="search.no_results_found" />
                </h3>
                <p class="max-w-md text-gray-500">
                  <Trans
                    key="search.no_results_description"
                    options={{ type: type() }}
                  />
                </p>
              </div>
            }
          >
            <VList
              data={searchContext?.allRows() || []}
              class="h-full flex max-w-full flex-col gap-4 overflow-auto px-6 pb-6"
              ref={(v) => {
                if (v) {
                  searchContext?.setRef(v)
                }
              }}
              onScroll={searchContext?.virtualOnScrollHandler}
            >
              {(result) => {
                if (result.type === "loader") {
                  return <Skeleton.searchListItem />
                }

                const isInstalled = createMemo(() =>
                  lookupTableInstalledMods().has(result.value!.id)
                )

                return (
                  <ListItem
                    instanceMods={instanceMods()?.data ?? undefined}
                    instanceId={instanceId()}
                    result={result.value!}
                    isInstalled={isInstalled()}
                    onItemClick={() => {
                      navigator.navigate(
                        `/addon/${result.value!.id}/${result.value!.platform}?instanceId=${instanceId()}`
                      )
                    }}
                  />
                )
              }}
            </VList>
          </Show>
        </Show>
      </div>
    </div>
  )
}

export default List
