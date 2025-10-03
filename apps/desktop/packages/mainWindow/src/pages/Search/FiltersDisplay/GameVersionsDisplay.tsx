import useSearchContext from "@/components/SearchInputContext"
import { For } from "solid-js"
import { FilterBadge } from "./FilterBadge"

export default function GameVersionsDisplay() {
  const searchContext = useSearchContext()

  return (
    <For each={searchContext?.searchQuery().gameVersions}>
      {(gameVersion) => {
        return (
          <FilterBadge
            onClick={() => {
              searchContext?.setSearchQuery((prev) => {
                const filteredGameVersions =
                  prev.gameVersions?.filter((g) => g !== gameVersion) || []
                return {
                  ...prev,
                  gameVersions:
                    filteredGameVersions.length === 0
                      ? null
                      : filteredGameVersions
                }
              })
            }}
          >
            {gameVersion}
          </FilterBadge>
        )
      }}
    </For>
  )
}
