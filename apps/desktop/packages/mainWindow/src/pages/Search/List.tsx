import { ListItem } from "./ListItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import {
  Badge,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  Tabs
} from "@gd/ui"
import { Tab, TabList } from "@gd/ui"
import { For, onMount, Suspense } from "solid-js"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useParams } from "@solidjs/router"
import FiltersDisplay from "./FiltersDisplay"
import { FiltersDropdown } from "./FiltersDropdown"

export const projectTypeTabs: {
  label: string
  value: FEUnifiedSearchType
  icon: string
  path: string
}[] = [
  {
    label: "Modpacks",
    value: "modpack",
    icon: "i-ri:folder-fill",
    path: "/search/modpack"
  },
  {
    label: "Mods",
    value: "mod",
    icon: "i-ri:file-text-fill",
    path: "/search/mod"
  },
  {
    label: "Shaders",
    value: "shader",
    icon: "i-ri:paint-fill",
    path: "/search/shader"
  },
  {
    label: "Resource Packs",
    value: "resourcePack",
    icon: "i-ri:folder-fill",
    path: "/search/resourcePack"
  },
  {
    label: "Data Packs",
    value: "datapack",
    icon: "i-ri:folder-fill",
    path: "/search/datapack"
  },
  {
    label: "Worlds",
    value: "world",
    icon: "i-ri:folder-fill",
    path: "/search/world"
  }
]

export function List() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const params = useParams()
  const type = () => params.type ?? "modpack"

  onMount(() => {
    queueMicrotask(() => {
      searchContext?.ref()?.scrollTo(searchContext.lastScrollOffset())
    })
  })

  return (
    <div class="flex h-full flex-col py-6">
      <FiltersDisplay />
      <div class="flex w-full justify-between px-6">
        <Tabs
          defaultIndex={projectTypeTabs.findIndex(
            (tab) => tab.value === type()
          )}
        >
          <TabList aligment="between">
            <For each={projectTypeTabs}>
              {(tab, index) => (
                <Tab
                  onClick={() => {
                    if (
                      index() ===
                      projectTypeTabs.findIndex((tab) => tab.value === type())
                    )
                      return

                    navigator.navigate(tab.path)

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
        <div>
          <DropdownMenu>
            <DropdownMenuTrigger>
              <Badge>
                <div class="flex items-center gap-1">
                  <div>Filters</div>
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
      <Suspense
        fallback={
          <div class="m-4 flex h-20 items-center justify-center">
            <div class="i-ri:loader-4-line animate-spin text-2xl" />
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
              return (
                <div class="m-4 flex h-20 items-center justify-center">
                  <div class="i-ri:loader-4-line animate-spin text-2xl" />
                </div>
              )
            }
            return (
              <ListItem
                result={result.value!}
                onItemClick={() => {
                  navigator.navigate(
                    `/addon/${result.value!.id}/${result.value!.platform}`
                  )
                }}
              />
            )
          }}
        </VList>
      </Suspense>
    </div>
  )
}

export default List
