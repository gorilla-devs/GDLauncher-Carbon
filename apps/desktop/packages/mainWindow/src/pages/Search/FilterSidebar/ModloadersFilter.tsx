import { Collapsable } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Show, createMemo } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { capitalize } from "@/utils/helpers"
import { ModloaderIcon } from "@/utils/sidebar"
import { FilterWarning } from "./shared"
import { SearchableCheckboxList } from "./SearchableCheckboxList"

export function ModloadersFilter() {
  const globalStore = useGlobalStore()
  const searchResults = useSearchContext()

  const modloaders = createMemo(
    () =>
      globalStore.modloaders.data?.map((modloader) => ({
        label: capitalize(modloader),
        value: modloader,
        icon: <ModloaderIcon modloader={modloader} />
      })) || []
  )

  const handleToggle = (value: string, checked: boolean) => {
    searchResults?.setSearchQuery((prev) => {
      const prevModloaders = prev.modloaders || []
      if (checked) {
        if (!prevModloaders.includes(value)) {
          return {
            ...prev,
            modloaders: [...prevModloaders, value]
          }
        }
      } else {
        const filtered = prevModloaders.filter((m) => m !== value)
        return {
          ...prev,
          modloaders: filtered.length === 0 ? null : filtered
        }
      }
      return prev
    })
  }

  return (
    <Collapsable title={<div class="flex items-center gap-2"><div class="i-hugeicons:puzzle h-4 w-4" /><Trans key="search:_trn_modloaders" /></div>} defaultOpened noPadding>
      <div class="flex flex-col gap-1 px-2">
        <Show when={!!searchResults?.selectedInstanceId()}>
          <FilterWarning />
        </Show>
        <SearchableCheckboxList
          items={modloaders()}
          selectedValues={() => searchResults?.searchQuery().modloaders || []}
          onToggle={handleToggle}
          searchPlaceholder="Search modloaders..."
          emptyMessage={<Trans key="content:_trn_common.no_modloaders_found" />}
        />
      </div>
    </Collapsable>
  )
}
