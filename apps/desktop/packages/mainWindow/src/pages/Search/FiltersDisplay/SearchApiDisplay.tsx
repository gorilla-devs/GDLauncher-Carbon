import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import { capitalize } from "@/utils/helpers"
import { PlatformLogo } from "@/components/PlatformLogo"

export default function SearchApiDisplay() {
  const searchContext = useSearchContext()

  return (
    <Show when={searchContext?.searchQuery().searchApi}>
      <FilterBadge
        onClick={() => {
          searchContext?.setSearchQuery((prev) => ({
            ...prev,
            searchApi: null
          }))
        }}
      >
        <div class="flex items-center gap-2">
          <PlatformLogo
            platform={
              searchContext?.searchQuery().searchApi === "curseforge"
                ? "curseforge"
                : "modrinth"
            }
            class="h-4 w-4"
          />
          {capitalize(searchContext?.searchQuery().searchApi)}
        </div>
      </FilterBadge>
    </Show>
  )
}
