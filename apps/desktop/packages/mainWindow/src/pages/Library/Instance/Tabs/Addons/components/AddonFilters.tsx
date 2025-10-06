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
import { AddonType, Mod } from "@gd/core_module/bindings"

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
    return t(`instance.tabs.${type}`)
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
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-1 items-center gap-4">
            <Input
              value={props.searchQuery()}
              onInput={(e) => props.setSearchQuery(e.target.value)}
              placeholder={t("instance.search_addons")}
              icon={<div class="i-ri:search-line" />}
              class="max-w-sm"
            />

            <div class="flex items-center gap-2">
              <span class="text-lightSlate-600 text-sm">
                {t("instance.platform_filter")}:
              </span>
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
                        return t("instance.filter.all")
                      case "curseforge":
                        return t("platforms.curseforge")
                      case "modrinth":
                        return t("platforms.modrinth")
                      case "local":
                        return t("instance.filter.local")
                      default:
                        return value
                    }
                  }
                  const getIcon = (value: string) => {
                    switch (value) {
                      case "all":
                        return <div class="i-ri:global-line w-4 h-4" />
                      case "curseforge":
                        return <div class="i-simple-icons:curseforge w-4 h-4" />
                      case "modrinth":
                        return <div class="i-simple-icons:modrinth w-4 h-4" />
                      case "local":
                        return <div class="i-ri:folder-line w-4 h-4" />
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
                <SelectTrigger class="w-40">
                  <SelectValue<string>>
                    {(state) => {
                      const getLabel = (value: string) => {
                        switch (value) {
                          case "all":
                            return t("instance.filter.all")
                          case "curseforge":
                            return t("platforms.curseforge")
                          case "modrinth":
                            return t("platforms.modrinth")
                          case "local":
                            return t("instance.filter.local")
                          default:
                            return value
                        }
                      }
                      const getIcon = (value: string) => {
                        switch (value) {
                          case "all":
                            return <div class="i-ri:global-line w-4 h-4" />
                          case "curseforge":
                            return (
                              <div class="i-simple-icons:curseforge w-4 h-4" />
                            )
                          case "modrinth":
                            return (
                              <div class="i-simple-icons:modrinth w-4 h-4" />
                            )
                          case "local":
                            return <div class="i-ri:folder-line w-4 h-4" />
                          default:
                            return null
                        }
                      }
                      const selectedValue = state.selectedOption()
                      return (
                        <div class="flex items-center gap-2">
                          {getIcon(selectedValue)}
                          {getLabel(selectedValue)}
                        </div>
                      )
                    }}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent />
              </Select>
            </div>
          </div>

          <div class="flex items-center gap-3">
            <Show when={props.updateCount() > 0}>
              <Tooltip open={props.isInstanceLocked() ? undefined : false}>
                <TooltipTrigger>
                  <Button
                    type="outline"
                    size="small"
                    onClick={props.onUpdateAll}
                    disabled={props.isInstanceLocked()}
                    class="text-xs"
                  >
                    <div class="i-ri:download-2-fill text-sm" />
                    <Trans
                      key="instance.update_all_count"
                      options={{ count: props.updateCount() }}
                    />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <Trans key="instance.locked_cannot_apply_changes" />
                </TooltipContent>
              </Tooltip>
            </Show>

            <Button
              type="secondary"
              size="small"
              onClick={props.onOpenFolder}
              class="px-3"
            >
              <div class="i-ri:folder-open-fill" />
            </Button>

            <Tooltip open={props.isInstanceLocked() ? undefined : false}>
              <TooltipTrigger>
                <Button
                  type="primary"
                  size="small"
                  onClick={props.onAddAddons}
                  disabled={props.isInstanceLocked()}
                  class="font-semibold"
                >
                  <Trans key="instance.add_addons" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="instance.locked_cannot_apply_changes" />
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* Addon type filters */}
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-wrap items-center gap-2">
            <div class="i-ri:filter-3-line text-lightSlate-600 w-5 h-5" />
            <For each={visibleAddonTypes()}>
              {(type) => (
                <Badge
                  variant={
                    props.enabledAddonTypes[type] ? "default" : "secondary"
                  }
                  class="cursor-pointer transition-colors"
                  onClick={() => {
                    props.setEnabledAddonTypes(
                      type,
                      !props.enabledAddonTypes[type]
                    )
                  }}
                >
                  {getAddonTypeLabel(type)}
                  <Show when={props.enabledAddonTypes[type]}>
                    <div class="i-ri:check-line ml-1" />
                  </Show>
                </Badge>
              )}
            </For>
          </div>
          <div class="text-lightSlate-600 text-xs flex items-center gap-2">
            <div class="i-ri:mouse-line" />
            <span>{t("instance.right_click_hint")}</span>
            <span class="text-lightSlate-700">•</span>
            <span>{t("instance.multi_select_hint")}</span>
          </div>
        </div>
      </div>
    </div>
  )
}
