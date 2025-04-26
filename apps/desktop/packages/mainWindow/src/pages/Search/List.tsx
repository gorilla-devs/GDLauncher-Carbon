import { ListItem } from "./ListItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import { Tabs } from "@gd/ui"
import { Tab, TabList } from "@gd/ui"
import { createSignal, For, onMount, Suspense } from "solid-js"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"

export function List() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()

  onMount(() => {
    queueMicrotask(() => {
      searchContext?.ref()?.scrollTo(searchContext.lastScrollOffset())
    })
  })

  const [selectedTab, setSelectedTab] = createSignal(0)
  const tabs: { label: string; value: FEUnifiedSearchType }[] = [
    {
      label: "Modpacks",
      value: "modpack"
    },
    {
      label: "Mods",
      value: "mod"
    },
    {
      label: "Shaders",
      value: "shader"
    },
    {
      label: "Resource Packs",
      value: "resourcePack"
    },
    {
      label: "Data Packs",
      value: "datapack"
    },
    {
      label: "Worlds",
      value: "world"
    }
  ]

  return (
    <div class="flex h-full flex-col">
      <div class="w-full p-4">
        <Tabs index={selectedTab()}>
          <TabList aligment="between">
            <For each={tabs}>
              {(tab, index) => (
                <Tab
                  onClick={() => {
                    if (index() === selectedTab()) return

                    setSelectedTab(index)
                    searchContext?.setSearchQuery((prev) => ({
                      ...prev,
                      projectType: tab.value
                    }))
                  }}
                >
                  {tab.label}
                </Tab>
              )}
            </For>
          </TabList>
        </Tabs>
      </div>
      <Suspense>
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
