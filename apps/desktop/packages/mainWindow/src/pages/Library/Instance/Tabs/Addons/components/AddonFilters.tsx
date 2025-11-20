import {
  Badge,
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { For, Show, onMount, onCleanup, createEffect } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { getAddonTabKey } from "@gd/i18n/helpers"
import { AddonType, Mod } from "@gd/core_module/bindings"
import { getAddonTypeIcon } from "@/utils/addonIcons"

const ADDON_TYPES: AddonType[] = [
  "mods",
  "shaders",
  "resourcepacks",
  "datapacks",
  "worlds"
]

interface AddonFiltersProps {
  searchQuery: () => string
  setSearchQuery: (query: string) => void
  enabledAddonTypes: Record<AddonType, boolean>
  setEnabledAddonTypes: (type: AddonType, enabled: boolean) => void
  platformFilter: () => "all" | "curseforge" | "modrinth" | "local"
  setPlatformFilter: (
    filter: "all" | "curseforge" | "modrinth" | "local"
  ) => void
  isInstanceLocked: () => boolean
  onAddAddons: () => void
  onOpenFolder: () => void
  onUpdateAll: () => void
  updateCount: () => number
  hasModloaders: () => boolean
  addons: () => Mod[]
  onHeightChange?: (height: number) => void
}

export const AddonFilters = (props: AddonFiltersProps) => {
  const [t] = useTransContext()
  let containerRef: HTMLDivElement | undefined

  const getAddonTypeLabel = (type: AddonType) => {
    return t(getAddonTabKey(type))
  }

  const visibleAddonTypes = () => {
    return ADDON_TYPES.filter((type) => {
      if (type === "mods" && !props.hasModloaders()) {
        return false
      }

      // Only show addon types that have at least one installed addon
      const hasAddonsOfType = props
        .addons()
        .some((addon) => addon.addon_type === type)
      return hasAddonsOfType
    })
  }

  const measureHeight = () => {
    if (containerRef && props.onHeightChange) {
      const height = containerRef.offsetHeight
      props.onHeightChange(height)
    }
  }

  onMount(() => {
    if (containerRef && props.onHeightChange) {
      // Initial measurement
      measureHeight()

      // Observe size changes
      const resizeObserver = new ResizeObserver(() => {
        measureHeight()
      })

      resizeObserver.observe(containerRef)

      onCleanup(() => {
        resizeObserver.disconnect()
      })
    }
  })

  // Also measure when visible addon types change (affects badge layout)
  createEffect(() => {
    visibleAddonTypes() // Track dependency
    const timeoutId = setTimeout(measureHeight, 0) // Defer to next tick to ensure layout is updated

    onCleanup(() => {
      clearTimeout(timeoutId)
    })
  })

  return (
    <div
      ref={containerRef}
      class="bg-darkSlate-800 border-darkSlate-700 sticky top-14 z-20 border-b px-6 py-4"
    >
      <div class="flex flex-col gap-4">
        {/* Search and main actions */}
        <div class="flex items-center gap-2">
          <Input
            value={props.searchQuery()}
            onInput={(e) => props.setSearchQuery(e.target.value)}
            placeholder={t("content:_trn_search_addons")}
            icon={<div class="i-hugeicons:search-01" />}
            class="hidden lg:flex flex-1 min-w-0"
          />

          <Select
            value={props.platformFilter()}
            onChange={(value) => value && props.setPlatformFilter(value)}
            options={["all", "curseforge", "modrinth", "local"]}
            placeholder=""
            disallowEmptySelection={true}
            selectionBehavior="replace"
            itemComponent={(itemProps) => {
              const getLabel = (value: string) => {
                switch (value) {
                  case "all":
                    return t("content:_trn_filter.all")
                  case "curseforge":
                    return t("enums:_trn_curseforge")
                  case "modrinth":
                    return t("enums:_trn_modrinth")
                  case "local":
                    return t("content:_trn_filter.local")
                  default:
                    return value
                }
              }
              const getIcon = (value: string) => {
                switch (value) {
                  case "all":
                    return <div class="i-hugeicons:globe h-4 w-4" />
                  case "curseforge":
                    return <div class="i-simple-icons:curseforge h-4 w-4" />
                  case "modrinth":
                    return <div class="i-simple-icons:modrinth h-4 w-4" />
                  case "local":
                    return <div class="i-hugeicons:folder-01 h-4 w-4" />
                  default:
                    return null
                }
              }
              return (
                <SelectItem item={itemProps.item}>
                  <div class="flex items-center gap-2">
                    {getIcon(itemProps.item.rawValue)}
                    {getLabel(itemProps.item.rawValue)}
                  </div>
                </SelectItem>
              )
            }}
          >
            <SelectTrigger class="w-32 md:w-40">
              <SelectValue<string>>
                {(state) => {
                  const getLabel = (value: string) => {
                    switch (value) {
                      case "all":
                        return t("content:_trn_filter.all")
                      case "curseforge":
                        return t("enums:_trn_curseforge")
                      case "modrinth":
                        return t("enums:_trn_modrinth")
                      case "local":
                        return t("content:_trn_filter.local")
                      default:
                        return value
                    }
                  }
                  const getIcon = (value: string) => {
                    switch (value) {
                      case "all":
                        return <div class="i-hugeicons:globe h-4 w-4" />
                      case "curseforge":
                        return <div class="i-simple-icons:curseforge h-4 w-4" />
                      case "modrinth":
                        return <div class="i-simple-icons:modrinth h-4 w-4" />
                      case "local":
                        return <div class="i-hugeicons:folder-01 h-4 w-4" />
                      default:
                        return null
                    }
                  }
                  const selectedValue = state.selectedOption()
                  return (
                    <div class="flex items-center gap-2">
                      {getIcon(selectedValue)}
                      <span class="hidden sm:inline">
                        {getLabel(selectedValue)}
                      </span>
                    </div>
                  )
                }}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>

          <Show when={props.updateCount() > 0}>
            <Tooltip>
              <TooltipTrigger>
                <Button
                  type="secondary"
                  size="small"
                  onClick={props.onUpdateAll}
                  disabled={props.isInstanceLocked()}
                  class="px-2"
                >
                  <div class="i-hugeicons:download-02" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Show
                  when={!props.isInstanceLocked()}
                  fallback={
                    <Trans key="instances:_trn_locked_cannot_apply_changes" />
                  }
                >
                  <Trans
                    key="content:_trn_update_all_count"
                    options={{ count: props.updateCount() }}
                  />
                </Show>
              </TooltipContent>
            </Tooltip>
          </Show>

          <Tooltip>
            <TooltipTrigger>
              <Button
                type="secondary"
                size="small"
                onClick={props.onOpenFolder}
                class="px-2"
              >
                <div class="i-hugeicons:folder-open" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <Trans key="instances:_trn_open_folder" />
            </TooltipContent>
          </Tooltip>

          <Tooltip open={props.isInstanceLocked() ? undefined : false}>
            <TooltipTrigger>
              <Button
                type="primary"
                size="small"
                onClick={props.onAddAddons}
                disabled={props.isInstanceLocked()}
                class="font-semibold"
              >
                <div class="i-hugeicons:add-01" />
                <span class="hidden md:inline">
                  <Trans key="content:_trn_add_addons" />
                </span>
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <Trans key="instances:_trn_locked_cannot_apply_changes" />
            </TooltipContent>
          </Tooltip>
        </div>

        {/* Addon type filters */}
        <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div class="flex flex-wrap items-center gap-2">
            <div class="text-lightSlate-600 h-5 w-5" />
            <For each={visibleAddonTypes()}>
              {(type) => (
                <Badge
                  variant={
                    props.enabledAddonTypes[type] ? "default" : "secondary"
                  }
                  class="flex cursor-pointer items-center gap-1.5 transition-colors"
                  onClick={() => {
                    props.setEnabledAddonTypes(
                      type,
                      !props.enabledAddonTypes[type]
                    )
                  }}
                >
                  <div class={`${getAddonTypeIcon(type)} text-sm`} />
                  <span class="hidden md:inline">
                    {getAddonTypeLabel(type)}
                  </span>
                  <Show when={props.enabledAddonTypes[type]}>
                    <div class="i-hugeicons:tick-02 ml-1" />
                  </Show>
                </Badge>
              )}
            </For>
          </div>
          <div class="text-lightSlate-600 hidden xl:flex items-center gap-2 text-xs flex-shrink-0">
            <div class="i-hugeicons:mouse-01" />
            <span>{t("content:_trn_right_click_hint")}</span>
            <span class="text-lightSlate-700">•</span>
            <span>{t("content:_trn_multi_select_hint")}</span>
          </div>
        </div>
      </div>
    </div>
  )
}
