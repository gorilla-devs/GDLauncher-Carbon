import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { capitalize } from "@/utils/helpers"

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
          <img
            src={
              searchContext?.searchQuery().searchApi === "curseforge"
                ? CurseforgeLogo
                : ModrinthLogo
            }
            class="h-4 w-4"
          />
          {capitalize(searchContext?.searchQuery().searchApi)}
        </div>
      </FilterBadge>
    </Show>
  )
}
