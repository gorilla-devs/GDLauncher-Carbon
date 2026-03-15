import { Collapsable } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Show, createMemo } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import useSearchContext from "@/components/SearchInputContext"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import { SearchableCheckboxList } from "./SearchableCheckboxList"

export function CategoriesFilter() {
  const searchResults = useSearchContext()
  const categories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.getUnifiedCategories"]
  }))

  const curseforgeCategories = createMemo(() =>
    Object.values(categories.data?.curseforge ?? {})
      .filter(
        (v) => v.projectType === searchResults?.searchQuery().projectType
      )
      .map((category) => ({
        label: category.name,
        value: category.id,
        icon: <img src={category.icon?.value ?? ""} class="h-4 w-4" />
      }))
  )

  const modrinthCategories = createMemo(() =>
    Object.values(categories.data?.modrinth ?? {})
      .filter(
        (v) => v.projectType === searchResults?.searchQuery().projectType
      )
      .map((category) => ({
        label: category.name,
        value: category.id,
        icon: (
          // eslint-disable-next-line solid/no-innerhtml
          <div class="h-4 w-4" innerHTML={category.icon?.value ?? ""} />
        )
      }))
  )

  const selectedApi = () => searchResults?.searchQuery().searchApi
  const isApiVisible = (api: "curseforge" | "modrinth") => {
    const selected = selectedApi()
    return !selected || selected === api
  }

  const selectedCount = () => searchResults?.searchQuery().categories?.length ?? 0

  const toggleCategory = (categoryValue: string, checked: boolean) => {
    searchResults?.setSearchQuery((prev) => {
      const updated = checked
        ? [...(prev.categories || []), categoryValue]
        : (prev.categories || []).filter((v) => v !== categoryValue)
      return {
        ...prev,
        categories: updated.length === 0 ? null : updated
      }
    })
  }

  return (
    <Collapsable
      title={<div class="flex items-center gap-2"><div class="i-hugeicons:folder-01 h-4 w-4" /><Trans key="search:_trn_categories" /></div>}
      defaultOpened
      noPadding
      count={selectedCount()}
      onClear={() => {
        searchResults?.setSearchQuery((prev) => ({
          ...prev,
          categories: null
        }))
      }}
    >
      <div class="flex flex-col gap-2 px-2">
        <Show when={isApiVisible("curseforge")}>
          <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2 px-1 py-1">
              <img src={CurseforgeLogo} class="h-3.5 w-3.5" />
              <span class="text-lightSlate-600 text-xs font-medium uppercase">
                <Trans key="enums:_trn_curseforge" />
              </span>
            </div>
            <SearchableCheckboxList
              items={curseforgeCategories()}
              selectedValues={() => searchResults?.searchQuery().categories || []}
              onToggle={toggleCategory}
              showSearch={false}
              emptyMessage={<Trans key="search:_trn_no_categories_found" />}
            />
          </div>
        </Show>

        <Show when={isApiVisible("curseforge") && isApiVisible("modrinth")}>
          <div class="bg-darkSlate-700/50 h-px" />
        </Show>

        <Show when={isApiVisible("modrinth")}>
          <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2 px-1 py-1">
              <img src={ModrinthLogo} class="h-3.5 w-3.5" />
              <span class="text-lightSlate-600 text-xs font-medium uppercase">
                <Trans key="enums:_trn_modrinth" />
              </span>
            </div>
            <SearchableCheckboxList
              items={modrinthCategories()}
              selectedValues={() => searchResults?.searchQuery().categories || []}
              onToggle={toggleCategory}
              showSearch={false}
              emptyMessage={<Trans key="search:_trn_no_categories_found" />}
            />
          </div>
        </Show>
      </div>
    </Collapsable>
  )
}
