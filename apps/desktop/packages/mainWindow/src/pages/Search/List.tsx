import { ListItem } from "./ListItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import { Skeleton } from "@gd/ui"
import { onMount, Show, createMemo } from "solid-js"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

export function List() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const params = useParams()

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
      <div class="flex-1 overflow-hidden pt-6">
        <Show
          when={!searchContext?.isInitialLoading()}
          fallback={<Skeleton.searchList />}
        >
          <Show
            when={(searchContext?.allRows() || []).length > 0}
            fallback={
              <div class="flex flex-col items-center justify-center px-6 py-16 text-center">
                <PlaceholderGorilla
                  size={12}
                  variant="Searching Gorilla - Magnifying Glass"
                />
                <h3 class="mb-2 mt-6 text-xl font-semibold text-gray-300">
                  <Trans key="search:_trn_no_results_found" />
                </h3>
                <p class="max-w-md text-gray-500">
                  <Trans
                    key="search:_trn_no_results_description"
                    options={{ type: type() }}
                  />
                </p>
              </div>
            }
          >
            <VList
              data={searchContext?.allRows() || []}
              class="flex h-full max-w-full flex-col gap-4 overflow-auto px-6 pb-6"
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

                return (
                  <ListItem
                    instanceMods={instanceMods()?.data ?? undefined}
                    instanceId={instanceId()}
                    result={result.value!}
                    isInstalled={lookupTableInstalledMods().has(
                      result.value!.id
                    )}
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
