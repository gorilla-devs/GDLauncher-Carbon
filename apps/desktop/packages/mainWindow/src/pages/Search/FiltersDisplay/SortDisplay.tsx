import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import { capitalize } from "@/utils/helpers"

export default function SortDisplay() {
  const searchContext = useSearchContext()

  return (
    <>
      <Show when={searchContext?.searchQuery().sortIndex}>
        <FilterBadge
          onClick={() => {
            searchContext?.setSearchQuery((prev) => ({
              ...prev,
              sortIndex: null
            }))
          }}
        >
          <div class="flex items-center gap-2">
            <div class="i-ri:sort-asc h-4 w-4" />
            Sort: {capitalize(searchContext?.searchQuery().sortIndex!)}
          </div>
        </FilterBadge>
      </Show>

      <Show when={searchContext?.searchQuery().sortOrder}>
        <FilterBadge
          onClick={() => {
            searchContext?.setSearchQuery((prev) => ({
              ...prev,
              sortOrder: null
            }))
          }}
        >
          <div class="flex items-center gap-2">
            <div
              class={`h-4 w-4 ${
                searchContext?.searchQuery().sortOrder === "ascending"
                  ? "i-ri:sort-asc"
                  : "i-ri:sort-desc"
              }`}
            />
            Order: {capitalize(searchContext?.searchQuery().sortOrder!)}
          </div>
        </FilterBadge>
      </Show>
    </>
  )
}