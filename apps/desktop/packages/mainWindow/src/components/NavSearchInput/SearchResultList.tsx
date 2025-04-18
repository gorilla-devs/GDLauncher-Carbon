import { Show } from "solid-js"
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
  return (
    <>
      <VList
        data={props.results}
        class="flex max-w-full flex-col gap-4 overflow-x-hidden px-4"
        ref={props.setRef}
        onScroll={props.onScroll}
      >
        {(result) => (
          <SearchResultListItem
            result={result}
            onItemClick={props.onItemClick}
          />
        )}
      </VList>

      {/* Loading indicator after the list */}
      {props.isLoading && props.results.length > 0 && (
        <div class="my-4 flex h-20 items-center justify-center">
          <div class="i-ri:loader-4-line animate-spin text-2xl" />
        </div>
      )}
    </>
  )
}
