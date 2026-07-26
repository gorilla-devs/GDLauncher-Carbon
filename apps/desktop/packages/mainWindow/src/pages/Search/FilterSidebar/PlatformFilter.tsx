import { Collapsable, Radio } from "@gd/ui"
import { Trans } from "@gd/i18n"
import useSearchContext from "@/components/SearchInputContext"
import { capitalize } from "@/utils/helpers"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"

interface PlatformFilterProps {
  /** Set only by the docked panel instance (see `FilterSidebar/index.tsx`'s
   *  `ExpandedPanel`, which is mounted twice at once — docked panel and
   *  hover flyout — so its subtree is always duplicated in the DOM, just
   *  toggled between the two via opacity/pointer-events rather than a real
   *  unmount). Anchoring both copies would give every test-id here two
   *  matches; only the docked copy (the default, `sidebarExpanded` starts
   *  `true`) renders one. */
  testAnchors?: boolean
}

export function PlatformFilter(props: PlatformFilterProps) {
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
      title={
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:globe-02 h-4 w-4" />
          <Trans key="search:_trn_platform" />
        </div>
      }
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
        {/* The div wrapper (not a prop on Radio) carries the anchor: Radio
            spreads unknown props onto its native `<input type="radio">`,
            which is visually hidden (`class="hidden"`) behind the clickable
            label it renders alongside — an anchor there would reach the DOM
            but never be clickable. */}
        <div
          data-testid={
            props.testAnchors ? "search-platform-curseforge" : undefined
          }
        >
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
        </div>
        <div
          data-testid={
            props.testAnchors ? "search-platform-modrinth" : undefined
          }
        >
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
      </div>
    </Collapsable>
  )
}
