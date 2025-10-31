import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import { capitalize } from "@/utils/helpers"

export default function EnvironmentDisplay() {
  const searchContext = useSearchContext()

  return (
    <Show when={searchContext?.searchQuery().environment}>
      <FilterBadge
        onClick={() => {
          searchContext?.setSearchQuery((prev) => ({
            ...prev,
            environment: null
          }))
        }}
      >
        <div class="flex items-center gap-2">
          <div
            class={
              "h-4 w-4 " +
              (searchContext?.searchQuery().environment === "server"
                ? "i-hugeicons:server-stack-01"
                : "i-hugeicons:computer")
            }
          />
          {capitalize(searchContext?.searchQuery().environment)}
        </div>
      </FilterBadge>
    </Show>
  )
}
