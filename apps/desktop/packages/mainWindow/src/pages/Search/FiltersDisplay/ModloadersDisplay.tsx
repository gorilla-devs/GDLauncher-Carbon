import useSearchContext from "@/components/SearchInputContext"
import { For } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import { getModloaderIcon } from "@/utils/sidebar"

export default function ModloadersDisplay() {
  const searchContext = useSearchContext()

  return (
    <For each={searchContext?.searchQuery().modloaders}>
      {(modloader) => {
        return (
          <FilterBadge
            onClick={() => {
              searchContext?.setSearchQuery((prev) => {
                const filteredModloaders = prev.modloaders?.filter((m) => m !== modloader) || []
                return {
                  ...prev,
                  modloaders: filteredModloaders.length === 0 ? null : filteredModloaders
                }
              })
            }}
          >
            <div class="flex items-center gap-1">
              <img class="h-4 w-4" src={getModloaderIcon(modloader)} />
              {modloader}
            </div>
          </FilterBadge>
        )
      }}
    </For>
  )
}
