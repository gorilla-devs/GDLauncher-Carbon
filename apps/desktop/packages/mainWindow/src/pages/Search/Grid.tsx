import { GridItem } from "./GridItem"
import { VList } from "@/components/VirtuaWrapper"
import useSearchContext from "@/components/SearchInputContext"
import { Skeleton } from "@gd/ui"
import {
  onMount,
  Show,
  createMemo,
  For,
  createSignal,
  onCleanup
} from "solid-js"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { Trans } from "@gd/i18n"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import { SearchResultItem } from "@/utils/platformSearch"

const ITEM_WIDTH = 320 // Approximate width of each grid item in pixels (larger for better visuals)
const GAP = 24 // Gap between items in pixels

export function Grid() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const params = useParams()

  const [containerWidth, setContainerWidth] = createSignal(800)
  let containerRef: HTMLDivElement | undefined

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
    // Set up ResizeObserver to track container width
    if (containerRef) {
      const resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          setContainerWidth(entry.contentRect.width)
        }
      })
      resizeObserver.observe(containerRef)
      onCleanup(() => resizeObserver.disconnect())

      // Initial width
      setContainerWidth(containerRef.clientWidth)
    }
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

  // Calculate items per row based on container width
  const itemsPerRow = createMemo(() => {
    const width = containerWidth()
    const cols = Math.max(2, Math.floor((width + GAP) / (ITEM_WIDTH + GAP)))
    return Math.min(cols, 6) // Cap at 6 columns
  })

  // Group items into rows for virtualization
  const rows = createMemo(() => {
    const items = searchContext?.allRows() || []
    const perRow = itemsPerRow()
    const result: SearchResultItem[][] = []

    for (let i = 0; i < items.length; i += perRow) {
      result.push(items.slice(i, i + perRow))
    }

    return result
  })

  // Custom scroll handler for row-based virtualization
  // The default virtualOnScrollHandler uses item-based indices, but we're using row-based
  const gridScrollHandler = (_index: number) => {
    const virtualizer = searchContext?.ref()

    if (!virtualizer || searchContext?.isLoading()) return

    // Get the row-based end index and convert to item index
    const endRowIndex = virtualizer.findEndIndex()
    const endItemIndex = (endRowIndex + 1) * itemsPerRow()
    const totalItems = searchContext?.allRows().length || 0

    // Load more when user reaches 25% from the end of current items
    const loadThreshold = Math.ceil(totalItems - totalItems * 0.25)

    if (endItemIndex >= loadThreshold && searchContext?.hasNextPage()) {
      // Fetch next page from both platforms if available
      if (searchContext?.cfInfiniteResults?.hasNextPage) {
        searchContext.cfInfiniteResults.fetchNextPage()
      }
      if (searchContext?.mrInfiniteResults?.hasNextPage) {
        searchContext.mrInfiniteResults.fetchNextPage()
      }
    }
  }

  // Grid item skeleton matching actual card design
  const GridItemSkeleton = () => (
    <div class="bg-darkSlate-800 relative aspect-square overflow-hidden rounded-2xl border border-white/5">
      {/* Image area skeleton */}
      <Skeleton class="h-full w-full rounded-none" />

      {/* Bottom overlay matching card design */}
      <div class="absolute inset-x-0 bottom-0 h-1/5 border-t border-white/10 bg-black/60 p-3">
        <Skeleton class="h-4 w-3/4 rounded" />
      </div>
    </div>
  )

  // Grid skeleton for loading state - matches actual grid layout
  const GridSkeleton = () => (
    <div class="px-6 pb-6 pt-0">
      <div
        class="grid gap-6"
        style={{
          "grid-template-columns": `repeat(${itemsPerRow()}, minmax(0, 1fr))`
        }}
      >
        <For each={new Array(itemsPerRow() * 3)}>
          {() => <GridItemSkeleton />}
        </For>
      </div>
    </div>
  )

  return (
    <div ref={containerRef} class="flex h-full flex-col overflow-hidden">
      <div class="flex-1 overflow-hidden">
        <Show
          when={!searchContext?.isInitialLoading()}
          fallback={<GridSkeleton />}
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
              data={rows()}
              class="flex h-full max-w-full flex-col gap-6 overflow-auto px-6 pb-6"
              ref={(v) => {
                if (v) {
                  searchContext?.setRef(v)
                  // Restore scroll position when VList mounts
                  queueMicrotask(() => {
                    v.scrollTo(searchContext?.lastScrollOffset() ?? 0)
                  })
                }
              }}
              onScroll={gridScrollHandler}
            >
              {(row) => (
                <div
                  class="mb-6 grid gap-6"
                  style={{
                    "grid-template-columns": `repeat(${itemsPerRow()}, minmax(0, 1fr))`
                  }}
                >
                  <For each={row}>
                    {(item) => {
                      if (item.type === "loader") {
                        return <GridItemSkeleton />
                      }

                      return (
                        <GridItem
                          instanceMods={instanceMods()?.data ?? undefined}
                          instanceId={instanceId()}
                          result={item.value!}
                          isInstalled={lookupTableInstalledMods().has(
                            item.value!.id
                          )}
                          onItemClick={() => {
                            navigator.navigate(
                              `/addon/${item.value!.id}/${item.value!.platform}?instanceId=${instanceId()}`
                            )
                          }}
                        />
                      )
                    }}
                  </For>
                </div>
              )}
            </VList>
          </Show>
        </Show>
      </div>
    </div>
  )
}

export default Grid
