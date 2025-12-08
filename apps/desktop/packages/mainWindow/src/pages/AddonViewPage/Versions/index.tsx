import { useSearchParams } from "@solidjs/router"
import VersionRow from "./VersionRow"
import { rspc } from "@/utils/rspcClient"
import { useInfiniteVersionsQuery } from "@/components/InfiniteScrollVersionsQueryWrapper"
import { useContext, For, Show } from "solid-js"
import { StickyHeaderHeightContext } from "@/pages/AddonViewPage"
import { Trans } from "@gd/i18n"
import { AddonContext } from "@/pages/AddonViewPage"
import { Button, Spinner, Skeleton } from "@gd/ui"
import { createVirtualizer } from "@tanstack/solid-virtual"

const Versions = () => {
  const [searchParams] = useSearchParams()
  const mod = useContext(AddonContext)
  const stickyHeaderHeight = useContext(StickyHeaderHeightContext)

  const infiniteQuery = useInfiniteVersionsQuery()

  const rows = () => infiniteQuery.allRows()

  const instanceId = () => {
    const id = parseInt(searchParams.instanceId, 10)
    return isNaN(id) ? undefined : id
  }

  const instanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId() ?? 0],
    enabled: instanceId() !== undefined && instanceId()! > 0
  }))

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId() ?? null],
    enabled: instanceId() !== undefined && instanceId()! > 0
  }))

  const installedMod = () => {
    for (const version of rows()) {
      if (instanceMods.data) {
        for (const mod of instanceMods.data) {
          if (
            mod.curseforge?.file_id.toString() === version?.fileId ||
            mod.modrinth?.version_id === version?.fileId
          ) {
            return {
              id: mod?.id,
              remoteId: version?.fileId.toString()
            }
          }
        }
      }
    }
  }

  let versionsContainerRef: HTMLDivElement | undefined

  const virtualizer = createVirtualizer({
    get count() {
      return rows().length
    },
    getScrollElement: () => {
      let element: HTMLElement | undefined = versionsContainerRef
      while (element?.parentElement) {
        const style = window.getComputedStyle(element.parentElement)
        if (
          style.overflow === "auto" ||
          style.overflow === "scroll" ||
          style.overflowY === "auto" ||
          style.overflowY === "scroll"
        ) {
          return element.parentElement
        }
        element = element.parentElement
      }
      return null
    },
    estimateSize: () => 70,
    overscan: 5
  })

  return (
    <div class="flex h-full flex-col" ref={versionsContainerRef}>
      <div
        class="bg-darkSlate-800 border-darkSlate-600 text-lightSlate-400 sticky z-20 mb-4 grid grid-cols-[4fr_130px_100px_120px_150px] gap-4 border-b px-6 pb-3 pt-4 text-xs font-medium uppercase tracking-wide"
        style={{ top: `${stickyHeaderHeight()}px` }}
      >
        <div>
          <Trans key="search:_trn_browser_table_headers.name" />
        </div>
        <div>
          <Trans key="search:_trn_browser_table_headers.published" />
        </div>
        <div>
          <Trans key="search:_trn_browser_table_headers.downloads" />
        </div>
        <div>
          <Trans key="search:_trn_browser_table_headers.type" />
        </div>
        <div class="text-right">
          <Trans key="content:_trn_actions" />
        </div>
      </div>
      <div class="flex-1 px-6">
        <Show
          when={mod?.data && rows().length > 0}
          fallback={
            <Show
              when={!mod?.data || infiniteQuery.isLoading}
              fallback={
                <div class="flex flex-col items-center justify-center py-16 text-center">
                  <Show when={infiniteQuery.infiniteQuery.error}>
                    <div class="i-hugeicons:alert-02 text-3xl mb-4 text-red-400" />
                    <h3 class="mb-2 text-lg font-semibold text-red-300">
                      <Trans key="content:_trn_error_loading_versions" />
                    </h3>
                    <Button
                      type="secondary"
                      size="small"
                      onClick={() => infiniteQuery.infiniteQuery.refetch()}
                    >
                      <div class="i-hugeicons:refresh mr-2 h-4 w-4" />
                      <Trans key="general:_trn_retry" />
                    </Button>
                  </Show>
                  <Show when={!infiniteQuery.infiniteQuery.error}>
                    <span class="text-lightSlate-400">
                      <Trans key="content:_trn_no_versions_found" />
                    </span>
                  </Show>
                </div>
              }
            >
              <Skeleton.addonVersionsTable />
            </Show>
          }
        >
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative"
            }}
          >
            <For each={virtualizer.getVirtualItems()}>
              {(virtualItem) => {
                const version = rows()[virtualItem.index]
                if (!version) return null

                if (
                  virtualItem.index >= rows().length - 5 &&
                  infiniteQuery.infiniteQuery.hasNextPage &&
                  !infiniteQuery.infiniteQuery.isFetchingNextPage
                ) {
                  infiniteQuery.infiniteQuery.fetchNextPage().catch(() => {})
                }

                return (
                  <div
                    style={{
                      position: "absolute",
                      top: "0",
                      left: "0",
                      width: "100%",
                      height: `${virtualItem.size}px`,
                      transform: `translateY(${virtualItem.start}px)`
                    }}
                    class="border-darkSlate-700 hover:bg-darkSlate-700/30 grid grid-cols-[4fr_130px_100px_120px_150px] gap-4 border-b py-2 transition-colors duration-150"
                    classList={{
                      "bg-green-500/5 border-green-500/20":
                        installedMod()?.remoteId.toString() ===
                        version?.fileId.toString()
                    }}
                  >
                    <VersionRow
                      project={mod?.data}
                      modVersion={version}
                      installedFile={installedMod()}
                      instanceId={instanceId()}
                      type={mod?.data?.type}
                      instanceMods={instanceMods.data || undefined}
                      instanceDetails={instanceDetails.data || undefined}
                    />
                  </div>
                )
              }}
            </For>
          </div>
          <Show when={infiniteQuery.infiniteQuery.isFetchingNextPage}>
            <div class="flex h-20 items-center justify-center">
              <Spinner class="h-8 w-8" />
            </div>
          </Show>
        </Show>
      </div>
    </div>
  )
}

export default Versions
