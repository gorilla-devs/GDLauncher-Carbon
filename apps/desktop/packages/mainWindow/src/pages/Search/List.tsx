import { ListItem } from "./ListItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import {
  Badge,
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  Skeleton,
  Tabs
} from "@gd/ui"
import { Tab, TabList } from "@gd/ui"
import { For, onMount, Show } from "solid-js"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useLocation, useParams, useSearchParams } from "@solidjs/router"
import FiltersDisplay from "./FiltersDisplay"
import { FiltersDropdown } from "./FiltersDropdown"
import { rspc } from "@/utils/rspcClient"
import { Trans, useTransContext } from "@gd/i18n"

export function List() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const params = useParams()
  const location = useLocation()
  const [searchParams] = useSearchParams()
  const [t] = useTransContext()

  const instanceId = () => Number.parseInt(searchParams.instanceId, 10)

  const defaultType = () => params.type || (instanceId() ? "mod" : "modpack")
  const type = () => (params.type ?? defaultType()) as FEUnifiedSearchType

  if (type() !== searchContext?.searchQuery().projectType) {
    searchContext?.setSearchQuery((prev) => ({
      ...prev,
      projectType: type()
    }))
  }

  const projectTypeTabs: () => {
    label: string
    value: FEUnifiedSearchType
    icon: string
    path: string
  }[] = () => [
    ...(instanceId()
      ? []
      : [
          {
            label: t("search.modpacks"),
            value: "modpack" as const,
            icon: "i-ri:folder-fill",
            path: "/search/modpack"
          }
        ]),
    {
      label: t("search.mods"),
      value: "mod",
      icon: "i-ri:file-text-fill",
      path: "/search/mod"
    },
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
    },
    {
      label: t("search.data_packs"),
      value: "datapack",
      icon: "i-ri:folder-fill",
      path: "/search/datapack"
    },
    {
      label: t("search.worlds"),
      value: "world",
      icon: "i-ri:folder-fill",
      path: "/search/world"
    }
  ]

  onMount(() => {
    queueMicrotask(() => {
      searchContext?.ref()?.scrollTo(searchContext.lastScrollOffset())
    })
  })

  const installedMods = rspc.createQuery(() => ({
    queryKey: [
      "instance.getInstanceMods",
      { instance_id: instanceId(), addon_type: null }
    ]
  }))

  const lookupTableInstalledMods: () => Set<string> = () => {
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
  }

  return (
    <div class="flex h-full flex-col pb-6">
      <FiltersDisplay />
      <div class="flex w-full justify-between p-6">
        <div
          class="w-44 items-center gap-2"
          classList={{
            hidden: !instanceId(),
            flex: !!instanceId()
          }}
        >
          <Button
            size="small"
            type="outline"
            onClick={() => {
              navigator.navigate(`/library/${instanceId()}/mods`)
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
              <Badge>
                <div class="flex items-center gap-1">
                  <div>
                    <Trans key="search.filters" />
                  </div>
                  <div class="i-ri:arrow-down-s-line text-xs" />
                </div>
              </Badge>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <FiltersDropdown
                disabled={!searchContext?.searchQuery().searchApi}
              />
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      <Show
        when={!searchContext?.isLoading()}
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
            class="flex max-w-full flex-col gap-4 overflow-x-hidden"
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

              const isInstalled = lookupTableInstalledMods().has(
                result.value!.id
              )

              return (
                <ListItem
                  result={result.value!}
                  isInstalled={isInstalled}
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
  )
}

export default List
