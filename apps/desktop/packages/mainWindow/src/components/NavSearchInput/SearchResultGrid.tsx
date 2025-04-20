import { For } from "solid-js"
import { VList } from "../VirtuaWrapper"
import { SearchResultGridItem } from "./SearchResultItem"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { VirtualizerHandle } from "virtua/lib/solid"

interface SearchResultGridProps {
  results: FEUnifiedSearchResult[]
  isLoading: boolean
  hasNextPage?: boolean
  onItemClick: (id: string, platform: string) => void
  onScroll: (event: number) => void
  setRef: (el: VirtualizerHandle | undefined) => void
}

export default function SearchResultGrid(props: SearchResultGridProps) {
  const gridRows = () => {
    const rows = props.results.reduce((acc, item, i) => {
      if (i % 3 === 0) {
        acc.push([])
      }
      acc[acc.length - 1].push(item)
      return acc
    }, [] as FEUnifiedSearchResult[][])

    if (props.isLoading && props.results.length > 0) {
      rows.push([{ id: "loader", type: "loader" } as any])
    }

    return rows
  }

  return (
    <div class="flex flex-col h-full">
      <VList
        data={gridRows()}
        class="flex max-w-full flex-col overflow-x-hidden px-4"
        ref={props.setRef}
        onScroll={props.onScroll}
      >
        {(row) => {
          if (row.length === 1 && row[0]?.type === "loader") {
            return (
              <div class="h-20 flex items-center justify-center my-4">
                <div class="i-ri:loader-4-line animate-spin text-2xl" />
              </div>
            )
          }

          return (
            <div class="flex w-full gap-2 py-2">
              <For each={row}>
                {(result) => (
                  <div class="flex-1">
                    <SearchResultGridItem
                      result={result}
                      onItemClick={props.onItemClick}
                    />
                  </div>
                )}
              </For>
            </div>
          )
        }}
      </VList>
    </div>
  )
}
