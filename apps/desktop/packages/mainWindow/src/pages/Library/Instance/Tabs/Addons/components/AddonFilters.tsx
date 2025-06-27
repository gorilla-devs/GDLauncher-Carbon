import { Badge, Button, Input } from "@gd/ui"
import { For, Show } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { AddonType } from "@gd/core_module/bindings"

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
}

export const AddonFilters = (props: AddonFiltersProps) => {
  const [t] = useTransContext()

  const getAddonTypeLabel = (type: AddonType) => {
    return t(`instance.tabs.${type}`)
  }

  return (
    <div class="bg-darkSlate-800 border-darkSlate-700 sticky top-0 z-20 border-b px-6 py-4">
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
              <select
                value={props.platformFilter()}
                onChange={(e) => props.setPlatformFilter(e.target.value as any)}
                class="bg-darkSlate-700 border-darkSlate-600 rounded border px-2 py-1 text-sm"
              >
                <option value="all">{t("instance.filter.all")}</option>
                <option value="curseforge">CurseForge</option>
                <option value="modrinth">Modrinth</option>
                <option value="local">{t("instance.filter.local")}</option>
              </select>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <Button
              type="outline"
              size="medium"
              onClick={props.onAddAddons}
              disabled={props.isInstanceLocked()}
            >
              <Trans key="instance.add_addons" />
            </Button>

            <Button size="medium" onClick={props.onOpenFolder}>
              <div class="i-ri:folder-open-fill" />
            </Button>
          </div>
        </div>

        {/* Addon type filters */}
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-lightSlate-600 text-sm">
            {t("instance.addon_types")}:
          </span>
          <For each={ADDON_TYPES}>
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
      </div>
    </div>
  )
}
