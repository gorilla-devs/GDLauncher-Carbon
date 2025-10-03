import useSearchContext from "@/components/SearchInputContext"
import { CategoryIcon } from "@/utils/instances"
import { For } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { FilterBadge } from "./FilterBadge"

export default function CategoriesDisplay() {
  const searchContext = useSearchContext()
  const globalStore = useGlobalStore()

  return (
    <For each={searchContext?.searchQuery().categories}>
      {(category) => {
        // Not the best... but at this point we have no way of knowing which platform the category belongs to
        const categoryData =
          globalStore.categories.data?.curseforge[category as number] ??
          globalStore.categories.data?.modrinth[category as string]

        return (
          <FilterBadge
            onClick={() => {
              searchContext?.setSearchQuery((prev) => ({
                ...prev,
                categories:
                  prev.categories?.filter((c) => c !== category) ?? null
              }))
            }}
          >
            <CategoryIcon
              type={categoryData?.icon?.type}
              value={categoryData?.icon?.value}
            />
            {categoryData?.name}
          </FilterBadge>
        )
      }}
    </For>
  )
}
