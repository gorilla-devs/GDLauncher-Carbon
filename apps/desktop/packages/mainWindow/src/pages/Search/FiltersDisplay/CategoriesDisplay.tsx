import useSearchContext from "@/components/SearchInputContext"
import { CategoryIcon } from "@/utils/instances"
import { For } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { FilterBadge } from "./FilterBadge"
import { formatModrinthCategory } from "@/utils/modrinthCategories"

export default function CategoriesDisplay() {
  const searchContext = useSearchContext()
  const globalStore = useGlobalStore()

  return (
    <For each={searchContext?.searchQuery().categories}>
      {(category) => {
        const projectType = searchContext?.searchQuery().projectType
        const categoryData =
          globalStore.categories.data?.curseforge[category as number] ??
          globalStore.categories.data?.modrinth[`${projectType}:${category}`]

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
            {categoryData?.platform === "modrinth"
              ? formatModrinthCategory(categoryData?.name)
              : categoryData?.name}
          </FilterBadge>
        )
      }}
    </For>
  )
}
