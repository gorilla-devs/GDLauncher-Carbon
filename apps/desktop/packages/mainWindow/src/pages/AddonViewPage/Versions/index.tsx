import { useSearchParams } from "@solidjs/router"
import VersionRow from "./VersionRow"
import { rspc } from "@/utils/rspcClient"
import { VList } from "@/components/VirtuaWrapper"
import { useInfiniteVersionsQuery } from "@/components/InfiniteScrollVersionsQueryWrapper"
import { createMemo, useContext } from "solid-js"
import { Trans } from "@gd/i18n"
import { AddonContext } from "@/pages/AddonViewPage"

interface VersionItem {
  type: "value" | "loader" | "header"
  value?: any
}

const Versions = () => {
  const [searchParams] = useSearchParams()
  const mod = useContext(AddonContext)

  const infiniteQuery = useInfiniteVersionsQuery()

  const rows = () => infiniteQuery.allRows()

  const instanceId = () => parseInt(searchParams.instanceId, 10)

  const instanceMods = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
  }))

  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
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

  const allItems = createMemo((): VersionItem[] => {
    const items: VersionItem[] = []

    // Add header
    items.push({ type: "header" })

    // Add version rows
    rows().forEach((version) => {
      items.push({ type: "value", value: version })
    })

    // Add loader if loading more data
    if (
      infiniteQuery.infiniteQuery.isFetchingNextPage ||
      infiniteQuery.isLoading
    ) {
      items.push({ type: "loader" })
    }

    return items
  })

  const virtualOnScrollHandler = () => {
    if (!infiniteQuery || infiniteQuery.isLoading) return

    const virtualizer = infiniteQuery.ref()
    if (!virtualizer) return

    // Check if we're near the bottom
    const endIndex = virtualizer.findEndIndex()
    const totalItems = allItems().length

    // Load more when user reaches 75% from the end of current items
    const loadThreshold = Math.ceil(totalItems - totalItems * 0.25)

    if (
      endIndex >= loadThreshold &&
      infiniteQuery.infiniteQuery.hasNextPage &&
      !infiniteQuery.infiniteQuery.isFetchingNextPage
    ) {
      infiniteQuery.infiniteQuery.fetchNextPage()
    }
  }

  return (
    <div class="flex h-full flex-col">
      <VList
        data={allItems()}
        class="flex-1"
        ref={(v) => {
          if (v) {
            infiniteQuery?.setRef(v)
          }
        }}
        onScroll={virtualOnScrollHandler}
      >
        {(item) => {
          if (item.type === "header") {
            const gridCols = "grid-cols-[5fr_130px_130px_100px_50px_200px]"
            return (
              <div class={`mb-8 grid ${gridCols}`}>
                <div>
                  <Trans key="browser_table_headers.name" />
                </div>
                <div>
                  <Trans key="browser_table_headers.published" />
                </div>
                <div>
                  <Trans key="browser_table_headers.downloads" />
                </div>
                <div>
                  <Trans key="browser_table_headers.type" />
                </div>
                <div>
                  <Trans key="browser_table_headers.details" />
                </div>
              </div>
            )
          }

          if (item.type === "loader") {
            return (
              <div class="m-4 flex h-20 items-center justify-center">
                <div class="i-ri:loader-4-line animate-spin text-2xl" />
              </div>
            )
          }

          // Render version row
          return (
            <div class="grid grid-cols-[5fr_130px_130px_100px_50px_200px]">
              <VersionRow
                project={mod?.data}
                modVersion={item.value}
                installedFile={installedMod()}
                instanceId={instanceId()}
                type={mod?.data?.type}
                instanceMods={instanceMods.data || undefined}
                instanceDetails={instanceDetails.data || undefined}
              />
            </div>
          )
        }}
      </VList>
    </div>
  )
}

export default Versions
