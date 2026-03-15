import { Collapsable, Radio } from "@gd/ui"
import { Trans } from "@gd/i18n"
import useSearchContext from "@/components/SearchInputContext"
import { capitalize } from "@/utils/helpers"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"

export function PlatformFilter() {
  const searchResults = useSearchContext()

  const handleSelect = (value: string | number | string[] | undefined) => {
    const platform = value as "curseforge" | "modrinth"
    if (platform === searchResults?.searchQuery().searchApi) {
      searchResults?.setSearchQuery((prev) => ({
        ...prev,
        searchApi: null,
        platformFilters: null
      }))
    } else {
      searchResults?.setSearchQuery((prev) => ({
        ...prev,
        searchApi: platform,
        // Reset categories and sort when switching platforms since they're platform-specific
        categories: null,
        platformFilters: null
      }))
    }
  }

  return (
    <Collapsable
      title={<div class="flex items-center gap-2"><div class="i-hugeicons:globe-02 h-4 w-4" /><Trans key="search:_trn_platform" /></div>}
      defaultOpened
      noPadding
      count={searchResults?.searchQuery().searchApi ? 1 : 0}
      onClear={() => {
        searchResults?.setSearchQuery((prev) => ({
          ...prev,
          searchApi: null,
          platformFilters: null
        }))
      }}
    >
      <div class="flex flex-col px-2">
        <Radio
          value="curseforge"
          checked={searchResults?.searchQuery().searchApi === "curseforge"}
          onChange={handleSelect}
          allowDeselect
        >
          <div class="flex items-center gap-2">
            <img src={CurseforgeLogo} class="h-4 w-4" />
            {capitalize("curseforge")}
          </div>
        </Radio>
        <Radio
          value="modrinth"
          checked={searchResults?.searchQuery().searchApi === "modrinth"}
          onChange={handleSelect}
          allowDeselect
        >
          <div class="flex items-center gap-2">
            <img src={ModrinthLogo} class="h-4 w-4" />
            {capitalize("modrinth")}
          </div>
        </Radio>
      </div>
    </Collapsable>
  )
}
