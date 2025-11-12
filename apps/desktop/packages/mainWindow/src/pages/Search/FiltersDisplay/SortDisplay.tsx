import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { Trans } from "@gd/i18n"
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

  const getCurseforgeSortField = () => {
    const filters = platformFilters()
    return (
      (filters?.platform === "curseforge" && filters.filters.sort_field) || null
    )
  }

  const getCurseforgeSortOrder = () => {
    const filters = platformFilters()
    return (
      (filters?.platform === "curseforge" && filters.filters.sort_order) || null
    )
  }

  const getModrinthSortIndex = () => {
    const filters = platformFilters()
    return (
      (filters?.platform === "modrinth" && filters.filters.sort_index) || null
    )
  }

  return (
    <>
      <Show when={getCurseforgeSortField()}>
        <FilterBadge onClick={clearPlatformFilters}>
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:sort-by-up-01 h-4 w-4" />
            <Trans
              key="search:_trn_sort_by"
              options={{ sort: capitalize(getCurseforgeSortField()) }}
            />
          </div>
        </FilterBadge>
      </Show>

      <Show when={getCurseforgeSortOrder()}>
        <FilterBadge onClick={clearPlatformFilters}>
          <div class="flex items-center gap-2">
            <div
              class={
                "h-4 w-4 " +
                (getCurseforgeSortOrder() === "ascending"
                  ? "i-hugeicons:sort-by-up-01"
                  : "i-hugeicons:sort-by-down-01")
              }
            />
            <Trans
              key="search:_trn_sort_order"
              options={{ order: capitalize(getCurseforgeSortOrder()) }}
            />
          </div>
        </FilterBadge>
      </Show>

      <Show when={getModrinthSortIndex()}>
        <FilterBadge onClick={clearPlatformFilters}>
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:sort-by-up-01 h-4 w-4" />
            <Trans
              key="search:_trn_sort_by"
              options={{ sort: capitalize(getModrinthSortIndex()) }}
            />
          </div>
        </FilterBadge>
      </Show>
    </>
  )
}
