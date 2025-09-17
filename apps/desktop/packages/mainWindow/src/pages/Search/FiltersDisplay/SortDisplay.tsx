import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import { capitalize } from "@/utils/helpers"

export default function SortDisplay() {
  const searchContext = useSearchContext()

  const platformFilters = () => searchContext?.searchQuery().platformFilters

  const clearPlatformFilters = () => {
    searchContext?.setSearchQuery((prev) => ({
      ...prev,
      platformFilters: null
    }))
  }

  return (
    <>
      <Show when={platformFilters()?.platform === "curseforge" && platformFilters()?.filters.sort_field}>
        <FilterBadge onClick={clearPlatformFilters}>
          <div class="flex items-center gap-2">
            <div class="i-ri:sort-asc h-4 w-4" />
            Sort: {capitalize(platformFilters()!.filters.sort_field!)}
          </div>
        </FilterBadge>
      </Show>

      <Show when={platformFilters()?.platform === "curseforge" && platformFilters()?.filters.sort_order}>
        <FilterBadge onClick={clearPlatformFilters}>
          <div class="flex items-center gap-2">
            <div
              class={`h-4 w-4 ${
                platformFilters()!.filters.sort_order === "ascending"
                  ? "i-ri:sort-asc"
                  : "i-ri:sort-desc"
              }`}
            />
            Order: {capitalize(platformFilters()!.filters.sort_order!)}
          </div>
        </FilterBadge>
      </Show>

      <Show when={platformFilters()?.platform === "modrinth" && platformFilters()?.filters.sort_index}>
        <FilterBadge onClick={clearPlatformFilters}>
          <div class="flex items-center gap-2">
            <div class="i-ri:sort-asc h-4 w-4" />
            Sort: {capitalize(platformFilters()!.filters.sort_index!)}
          </div>
        </FilterBadge>
      </Show>
    </>
  )
}
