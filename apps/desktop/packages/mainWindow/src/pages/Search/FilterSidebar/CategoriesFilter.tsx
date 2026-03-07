import { Collapsable } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Show } from "solid-js"
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

  const getCurseforgeCategories = () => {
    return Object.values(categories.data?.curseforge ?? {})
      ?.filter(
        (v) => v.projectType === searchResults?.searchQuery().projectType
      )
      .map((category) => ({
        label: category.name,
        value: category.id,
        icon: <img src={category.icon?.value ?? ""} class="h-4 w-4" />
      }))
  }

  const getModrinthCategories = () => {
    return Object.values(categories.data?.modrinth ?? {})
      ?.filter(
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
  }

  const selectedApi = () => searchResults?.searchQuery().searchApi
  const isApiVisible = (api: "curseforge" | "modrinth") => {
    const selected = selectedApi()
    return !selected || selected === api
  }

  const toggleCategory = (categoryValue: string, checked: boolean) => {
    searchResults?.setSearchQuery((prev) => ({
      ...prev,
      categories: checked
        ? [...(prev.categories || []), categoryValue]
        : (prev.categories || []).filter((v) => v !== categoryValue)
    }))
  }

  return (
    <Collapsable title={<div class="flex items-center gap-2"><div class="i-hugeicons:folder-01 h-4 w-4" /><Trans key="search:_trn_categories" /></div>} defaultOpened noPadding>
      <div class="flex flex-col gap-1">
        <Show when={isApiVisible("curseforge")}>
          <Collapsable
            title={
              <div class="flex items-center gap-2">
                <img src={CurseforgeLogo} class="h-4 w-4" />
                <Trans key="enums:_trn_curseforge" />
              </div>
            }
            size="small"
            defaultOpened
            noPadding
          >
            <div class="pl-2">
              <SearchableCheckboxList
                items={getCurseforgeCategories()}
                selectedValues={() => searchResults?.searchQuery().categories || []}
                onToggle={toggleCategory}
                showSearch={false}
                emptyMessage={<Trans key="search:_trn_no_categories_found" />}
              />
            </div>
          </Collapsable>
        </Show>

        <Show when={isApiVisible("modrinth")}>
          <Collapsable
            title={
              <div class="flex items-center gap-2">
                <img src={ModrinthLogo} class="h-4 w-4" />
                <Trans key="enums:_trn_modrinth" />
              </div>
            }
            size="small"
            defaultOpened
            noPadding
          >
            <div class="pl-2">
              <SearchableCheckboxList
                items={getModrinthCategories()}
                selectedValues={() => searchResults?.searchQuery().categories || []}
                onToggle={toggleCategory}
                showSearch={false}
                emptyMessage={<Trans key="search:_trn_no_categories_found" />}
              />
            </div>
          </Collapsable>
        </Show>
      </div>
    </Collapsable>
  )
}
