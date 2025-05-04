import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { Badge } from "@gd/ui"
import { For, Show } from "solid-js"

export default function FiltersDisplay() {
  const searchContext = useSearchContext()
  const globalStore = useGlobalStore()

  return (
    <Show when={searchContext?.searchQuery().categories?.length}>
      <div class="p-6">
        <div class="flex gap-2">
          <For each={searchContext?.searchQuery().categories}>
            {(category) => (
              <Badge
                onClick={(e) => {
                  e.preventDefault()
                  e.stopPropagation()
                  searchContext?.setSearchQuery((prev) => ({
                    ...prev,
                    categories:
                      prev.categories?.filter((c) => c !== category) ?? null
                  }))
                }}
              >
                {/* TODO: fix type */}
                {globalStore.categories.data?.curseforge[category as any]?.name}
              </Badge>
            )}
          </For>
        </div>
      </div>
    </Show>
  )
}
