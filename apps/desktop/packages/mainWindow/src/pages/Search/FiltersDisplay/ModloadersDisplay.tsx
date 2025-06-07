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
              searchContext?.setSearchQuery((prev) => ({
                ...prev,
                modloaders:
                  prev.modloaders?.filter((m) => m !== modloader) ?? null
              }))
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
