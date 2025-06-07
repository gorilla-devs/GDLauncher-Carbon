import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import InstanceDisplay from "./InstanceDisplay"
import CategoriesDisplay from "./CategoriesDisplay"
import ModloadersDisplay from "./ModloadersDisplay"
import GameVersionsDisplay from "./GameVersionsDisplay"

export default function FiltersDisplay() {
  const searchContext = useSearchContext()

  return (
    <Show
      when={
        searchContext?.searchQuery().categories?.length ||
        searchContext?.selectedInstanceId()
      }
    >
      <div class="px-6 pt-6">
        <div class="flex items-center gap-4">
          <div>Active Filters:</div>
          <InstanceDisplay />
          <CategoriesDisplay />
          <ModloadersDisplay />
          <GameVersionsDisplay />
        </div>
      </div>
    </Show>
  )
}
