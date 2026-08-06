import { createMemo, For } from "solid-js"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from "@gd/ui"
import { FEUnifiedSearchType } from "@gd/core_module/bindings"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useLocation } from "@solidjs/router"
import useSearchContext from "./SearchInputContext"
import { useTransContext } from "@gd/i18n"
import { getAddonTypeIcon } from "@/utils/addonIcons"

interface AddonTypeOption {
  label: string
  value: FEUnifiedSearchType
  icon: string
  path: string
}

// The addon types users can actually browse. `FEUnifiedSearchType` also carries
// "plugin" and "unknown", which have no label and are never offered as options.
const LABEL_KEYS = {
  modpack: "search:_trn_modpacks",
  mod: "search:_trn_mods",
  shader: "search:_trn_shaders",
  resourcePack: "search:_trn_resource_packs",
  datapack: "search:_trn_data_packs",
  world: "search:_trn_worlds"
} as const

type BrowsableAddonType = keyof typeof LABEL_KEYS

export function AddonTypeDropdown() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const location = useLocation()
  const [t] = useTransContext()

  // Which types are offered depends on what the search is adding to (nothing,
  // an instance, or a server); the search context owns that rule.
  const addonTypeOptions: () => AddonTypeOption[] = () =>
    (searchContext?.allowedAddonTypes() ?? [])
      .filter((value): value is BrowsableAddonType => value in LABEL_KEYS)
      .map((value) => ({
        label: t(LABEL_KEYS[value]),
        value,
        icon: getAddonTypeIcon(value),
        path: `/search/${value}`
      }))

  const currentType = () => searchContext?.searchQuery().projectType

  const currentOption = createMemo(() => {
    return addonTypeOptions().find((opt) => opt.value === currentType())
  })

  const handleTypeChange = (option: AddonTypeOption) => {
    if (option.value === currentType()) return

    navigator.navigate(`${option.path}${location.search}`)

    queueMicrotask(() => {
      searchContext?.setSearchQuery((prev) => ({
        ...prev,
        projectType: option.value
      }))
    })
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        data-testid="addon-type-dropdown-trigger"
        class="text-lightSlate-50 hover:bg-darkSlate-600 data-[expanded]:bg-darkSlate-600 flex items-center gap-2 rounded px-2.5 py-1.5 transition-colors hover:text-white data-[expanded]:text-white"
      >
        <div class={`${currentOption()?.icon} text-lg`} />
        <span class="text-sm font-medium">{currentOption()?.label}</span>
        <div class="i-hugeicons:arrow-down-01 text-xs shrink-0" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <For each={addonTypeOptions()}>
          {(option) => (
            <DropdownMenuItem
              data-testid="addon-type-dropdown-option"
              data-addon-type={option.value}
              onClick={() => handleTypeChange(option)}
              class="flex items-center gap-2"
              classList={{
                "bg-darkSlate-600": option.value === currentType()
              }}
            >
              <div class={`${option.icon} text-lg`} />
              <span>{option.label}</span>
            </DropdownMenuItem>
          )}
        </For>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
