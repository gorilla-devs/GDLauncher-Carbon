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
              searchContext?.setSearchQuery((prev) => ({
                ...prev,
                gameVersions:
                  prev.gameVersions?.filter((g) => g !== gameVersion) ?? null
              }))
            }}
          >
            {gameVersion}
          </FilterBadge>
        )
      }}
    </For>
  )
}
