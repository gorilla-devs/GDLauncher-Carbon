import { VList } from "../VirtuaWrapper"
import { SearchResultListItem } from "./SearchResultItem"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { VirtualizerHandle } from "virtua/lib/solid"

interface SearchResultListProps {
  results: FEUnifiedSearchResult[]
  isLoading: boolean
  hasNextPage?: boolean
  onItemClick: (id: string, platform: string) => void
  onScroll: (event: number) => void
  setRef: (el: VirtualizerHandle | undefined) => void
}

export default function SearchResultList(props: SearchResultListProps) {
  // Create a special loader item that will be added to the list when loading
  const loaderItem = { id: "loader", type: "loader" }

  // Combine actual results with loader item when loading
  const listData = () => {
    if (props.isLoading) {
      return [...props.results, loaderItem as any]
    }
    return props.results
  }

  return (
    <div class="flex h-full flex-col">
      <VList
        data={listData()}
        class="flex max-w-full flex-col gap-4 overflow-x-hidden px-4"
        ref={props.setRef}
        onScroll={props.onScroll}
      >
        {(result) => {
          if (result.type === "loader") {
            return (
              <div class="my-4 flex h-20 items-center justify-center">
                <div class="i-ri:loader-4-line animate-spin text-2xl" />
              </div>
            )
          }
          return (
            <SearchResultListItem
              result={result}
              onItemClick={props.onItemClick}
            />
          )
        }}
      </VList>
    </div>
  )
}
