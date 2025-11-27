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
import { rspc } from "@/utils/rspcClient"
import { useTransContext } from "@gd/i18n"
import { getAddonTypeIcon } from "@/utils/addonIcons"

interface AddonTypeOption {
  label: string
  value: FEUnifiedSearchType
  icon: string
  path: string
}

export function AddonTypeDropdown() {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()
  const location = useLocation()
  const [t] = useTransContext()

  const instanceId = () => searchContext?.selectedInstanceId() || NaN

  const instance = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()],
    enabled: !isNaN(instanceId()) && instanceId() > 0
  }))

  const addonTypeOptions: () => AddonTypeOption[] = () => {
    let options: AddonTypeOption[] = []

    if (!instanceId()) {
      options.push({
        label: t("search:_trn_modpacks"),
        value: "modpack",
        icon: getAddonTypeIcon("modpack"),
        path: "/search/modpack"
      })
    }

    if (
      !instanceId() ||
      (instanceId() && (instance.data?.modloaders?.length ?? 0) > 0)
    ) {
      options.push({
        label: t("search:_trn_mods"),
        value: "mod",
        icon: getAddonTypeIcon("mod"),
        path: "/search/mod"
      })
    }

    options = options.concat([
      {
        label: t("search:_trn_shaders"),
        value: "shader",
        icon: getAddonTypeIcon("shader"),
        path: "/search/shader"
      },
      {
        label: t("search:_trn_resource_packs"),
        value: "resourcePack",
        icon: getAddonTypeIcon("resourcePack"),
        path: "/search/resourcePack"
      },
      {
        label: t("search:_trn_data_packs"),
        value: "datapack",
        icon: getAddonTypeIcon("datapack"),
        path: "/search/datapack"
      },
      {
        label: t("search:_trn_worlds"),
        value: "world",
        icon: getAddonTypeIcon("world"),
        path: "/search/world"
      }
    ])

    return options
  }

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
      <DropdownMenuTrigger class="text-lightSlate-50 hover:bg-darkSlate-600 data-[expanded]:bg-darkSlate-600 flex items-center gap-2 rounded px-2.5 py-1.5 transition-colors hover:text-white data-[expanded]:text-white">
        <div class={`${currentOption()?.icon} text-lg`} />
        <span class="text-sm font-medium">{currentOption()?.label}</span>
        <div class="i-hugeicons:arrow-down-01 text-xs shrink-0" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <For each={addonTypeOptions()}>
          {(option) => (
            <DropdownMenuItem
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
