import {
  Badge,
  Checkbox,
  Switch,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  Spinner
} from "@gd/ui"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { Show, createSignal } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { getAddonTabKey, getPlatformKey } from "@gd/i18n/helpers"
import { createColumnHelper } from "@tanstack/solid-table"
import { Mod as ModType } from "@gd/core_module/bindings"
import CopyIcon from "@/components/CopyIcon"
import { getModImageUrl } from "@/utils/instances"
import { getAddonTypeIcon } from "@/utils/addonIcons"

interface ColumnConfig {
  isInstanceLocked: () => boolean
  selectedCount: () => number
  totalRows: () => number
  onSelectAll: () => void
  onToggleMod: (mod: ModType) => Promise<void>
  onUpdateMod: (mod: ModType) => Promise<void>
  onDeleteMod: (mod: ModType) => Promise<void>
  onSwitchVersion: (mod: ModType) => void
  isModUpdating: (modId: string) => boolean
  instanceId: number
}

export const createAddonColumns = (config: ColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<ModType>()

  return [
    // Selection column
    columnHelper.display({
      id: "select",
      size: 32,
      header: () => {
        const isAllSelected = () =>
          config.selectedCount() > 0 &&
          config.selectedCount() === config.totalRows()
        const isSomeSelected = () =>
          config.selectedCount() > 0 &&
          config.selectedCount() < config.totalRows()

        return (
          <Checkbox
            checked={isAllSelected()}
            indeterminate={isSomeSelected()}
            onChange={(_checked) => {
              config.onSelectAll()
            }}
          />
        )
      },
      cell: (props) => (
        <div
          class={`transition-opacity duration-100 ease-spring ${
            props.row.getIsSelected()
              ? "opacity-100"
              : "opacity-0 group-hover/row:opacity-100"
          }`}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <Checkbox
            checked={props.row.getIsSelected()}
            disabled={!props.row.getCanSelect()}
            indeterminate={props.row.getIsSomeSelected()}
            onChange={(value) => props.row.toggleSelected(!!value)}
          />
        </div>
      ),
      enableSorting: false,
      enableHiding: false
    }),

    // Icon column
    columnHelper.display({
      id: "icon",
      size: 48,
      header: "",
      cell: (props) => {
        const mod = props.row.original

        const getModImage = () => {
          if (mod.curseforge?.has_image) {
            return getModImageUrl(
              config.instanceId.toString(),
              mod.id,
              "curseforge"
            )
          } else if (mod.modrinth?.has_image) {
            return getModImageUrl(
              config.instanceId.toString(),
              mod.id,
              "modrinth"
            )
          } else if (mod.metadata?.has_image) {
            return getModImageUrl(
              config.instanceId.toString(),
              mod.id,
              "metadata"
            )
          }
          return null
        }

        const imageUrl = getModImage()

        return (
          <div class="flex items-center justify-center">
            <Show
              when={imageUrl}
              fallback={
                <div class="bg-darkSlate-600 flex h-8 w-8 items-center justify-center rounded">
                  <div class="i-hugeicons:file-01 text-lg text-lightSlate-400" />
                </div>
              }
            >
              <div class="bg-darkSlate-600 flex h-8 w-8 items-center justify-center overflow-hidden rounded">
                <img
                  src={imageUrl!}
                  class="h-full w-full object-cover"
                  alt={mod.metadata?.name || mod.filename}
                  loading="lazy"
                />
              </div>
            </Show>
          </div>
        )
      },
      enableSorting: false,
      enableHiding: false
    }),

    // Filename column
    columnHelper.accessor("filename", {
      header: t("content:_trn_table.filename"),
      sortingFn: (rowA, rowB) => {
        const modA = rowA.original
        const modB = rowB.original
        const a = modA.metadata?.name || modA.filename
        const b = modB.metadata?.name || modB.filename
        return a.localeCompare(b, undefined, {
          sensitivity: "base",
          numeric: true
        })
      },
      cell: (info) => {
        const mod = info.row.original
        const displayName = mod.metadata?.name || mod.filename
        const [showCopy, setShowCopy] = createSignal(false)

        return (
          <div
            class="group flex min-w-0 items-center gap-2"
            onMouseEnter={() => setShowCopy(true)}
            onMouseLeave={() => setShowCopy(false)}
          >
            <Tooltip>
              <TooltipTrigger class="block w-full">
                <div class="flex min-w-0 flex-1 flex-col">
                  <div class="flex min-w-0 items-center gap-2">
                    <div class="min-w-0 flex-1 truncate text-left font-medium">
                      {displayName}
                    </div>
                    <div
                      class="transition-opacity duration-200 flex-shrink-0"
                      classList={{
                        "opacity-0 invisible": !showCopy(),
                        "opacity-100 visible": showCopy()
                      }}
                      onMouseDown={(e) => e.stopPropagation()}
                    >
                      <CopyIcon text={displayName} />
                    </div>
                  </div>
                  <Show when={mod.metadata?.name}>
                    <div class="flex min-w-0 items-center gap-2">
                      <div class="min-w-0 flex-1 truncate text-left text-lightSlate-600 text-sm">
                        {mod.filename}
                      </div>
                      <div class="flex-shrink-0 w-4" />
                    </div>
                  </Show>
                </div>
              </TooltipTrigger>
              <TooltipContent>
                <div class="flex flex-col gap-1">
                  <div class="font-medium">{displayName}</div>
                  <Show when={mod.metadata?.name}>
                    <div class="text-lightSlate-400 text-xs">
                      {mod.filename}
                    </div>
                  </Show>
                </div>
              </TooltipContent>
            </Tooltip>
          </div>
        )
      }
    }),

    // Duplicate Warning column
    columnHelper.accessor("is_duplicate", {
      header: "",
      size: 48,
      cell: (props) => {
        const mod = props.row.original
        return (
          <Show when={mod.is_duplicate}>
            <Tooltip>
              <TooltipTrigger>
                <div class="i-hugeicons:alert-01 text-lg text-yellow-500 hidden lg:flex" />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="content:_trn_duplicate_mod_warning" />
              </TooltipContent>
            </Tooltip>
          </Show>
        )
      },
      enableSorting: false
    }),

    // Type column
    columnHelper.display({
      id: "type",
      header: () => (
        <span class="hidden lg:inline">{t("content:_trn_table.type")}</span>
      ),
      size: 104,
      cell: (props) => {
        const mod = props.row.original
        return (
          <div class="hidden lg:flex">
            <Badge variant="secondary" class="flex items-center gap-1.5">
              <div
                class={`${getAddonTypeIcon(mod.addon_type)} shrink-0 text-base`}
              />
              {t(getAddonTabKey(mod.addon_type))}
            </Badge>
          </div>
        )
      }
    }),

    // Platform column
    columnHelper.display({
      id: "platform",
      header: () => (
        <span class="hidden md:inline">{t("content:_trn_table.platform")}</span>
      ),
      size: 78,
      cell: (props) => {
        const mod = props.row.original
        const hasCurseforge = !!mod.curseforge
        const hasModrinth = !!mod.modrinth
        const hasBoth = hasCurseforge && hasModrinth

        // If no platforms, show local
        if (!hasCurseforge && !hasModrinth) {
          return (
            <Tooltip>
              <TooltipTrigger>
                <div class="i-hugeicons:folder-01 text-lg text-gray-500 hidden md:flex" />
              </TooltipTrigger>
              <TooltipContent>{t("content:_trn_table.local")}</TooltipContent>
            </Tooltip>
          )
        }

        // Single platform
        if (!hasBoth) {
          const platform = hasCurseforge ? "curseforge" : "modrinth"
          const logo = hasCurseforge ? CurseforgeLogo : ModrinthLogo
          return (
            <Tooltip>
              <TooltipTrigger>
                <img
                  src={logo}
                  class="h-4 w-4 hidden md:block"
                  alt={t(getPlatformKey(platform))}
                />
              </TooltipTrigger>
              <TooltipContent>{t(getPlatformKey(platform))}</TooltipContent>
            </Tooltip>
          )
        }

        // Both platforms - diagonal arrangement
        return (
          <div class="relative h-6 w-6 hidden md:block">
            <Tooltip>
              <TooltipTrigger>
                <div class="bg-darkSlate-800 absolute -left-0.5 -top-0.5 h-4 w-4 rounded-full p-0.5">
                  <img
                    src={CurseforgeLogo}
                    class="h-full w-full"
                    alt={t("enums:_trn_curseforge")}
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent>{t("enums:_trn_curseforge")}</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger>
                <div class="bg-darkSlate-800 absolute bottom-0.5 right-0.5 h-4 w-4 rounded-full p-0.5 shadow-sm">
                  <img
                    src={ModrinthLogo}
                    class="h-full w-full"
                    alt={t("enums:_trn_modrinth")}
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent>{t("enums:_trn_modrinth")}</TooltipContent>
            </Tooltip>
          </div>
        )
      }
    }),

    // Update Available column
    columnHelper.accessor("has_update", {
      header: t("content:_trn_table.update"),
      size: 80,
      cell: (props) => {
        const mod = props.row.original
        const isUpdating = () => config.isModUpdating(mod.id)

        const handleUpdate = async () => {
          if (isUpdating() || config.isInstanceLocked()) return
          await config.onUpdateMod(mod)
        }

        const isDisabled = () => isUpdating() || config.isInstanceLocked()

        return (
          <Show
            when={mod.has_update || isUpdating()}
            fallback={<span class="text-lightSlate-600 text-center">-</span>}
          >
            <Tooltip>
              <TooltipTrigger>
                <Show
                  when={!isUpdating()}
                  fallback={<Spinner class="h-5 w-5 text-blue-400" />}
                >
                  <div
                    class="i-hugeicons:download-02 text-lg cursor-pointer transition-colors"
                    classList={{
                      "text-green-500 hover:text-green-400": !isDisabled(),
                      "text-gray-400 cursor-not-allowed": isDisabled()
                    }}
                    onMouseDown={(e) => e.stopPropagation()}
                    onClick={handleUpdate}
                  />
                </Show>
              </TooltipTrigger>
              <TooltipContent>
                <Show
                  when={config.isInstanceLocked()}
                  fallback={
                    <Show
                      when={isUpdating()}
                      fallback={<Trans key="content:_trn_update_mod" />}
                    >
                      <Trans key="general:_trn_updating" />
                    </Show>
                  }
                >
                  <Trans key="instances:_trn_locked_cannot_apply_changes" />
                </Show>
              </TooltipContent>
            </Tooltip>
          </Show>
        )
      }
    }),

    // Status/Enable column
    columnHelper.accessor("enabled", {
      header: () => (
        <span class="hidden md:inline">{t("content:_trn_table.status")}</span>
      ),
      size: 100,
      cell: (props) => {
        const mod = props.row.original
        return (
          <div class="hidden md:flex">
            <Show
              when={!config.isInstanceLocked()}
              fallback={
                <Tooltip>
                  <TooltipTrigger>
                    <Switch checked={props.getValue()} disabled />
                  </TooltipTrigger>
                  <TooltipContent>
                    <Trans key="instances:_trn_locked_cannot_apply_changes" />
                  </TooltipContent>
                </Tooltip>
              }
            >
              <div class="group" onMouseDown={(e) => e.stopPropagation()}>
                <Switch
                  checked={mod.enabled}
                  onChange={() => config.onToggleMod(mod)}
                />
              </div>
            </Show>
          </div>
        )
      }
    }),

    // Actions column
    columnHelper.display({
      id: "actions",
      header: () => (
        <span class="hidden lg:inline">{t("content:_trn_table.actions")}</span>
      ),
      size: 80,
      cell: (props) => {
        const mod = props.row.original
        const hasPlatformData = !!mod.curseforge || !!mod.modrinth

        return (
          <Show when={hasPlatformData}>
            <div
              class="hidden lg:flex items-center justify-center"
              onMouseDown={(e) => e.stopPropagation()}
            >
              <Tooltip>
                <TooltipTrigger>
                  <div
                    class="i-hugeicons:arrow-left-right text-lg text-lightSlate-400 hover:text-lightSlate-200 cursor-pointer transition-colors"
                    onClick={() => config.onSwitchVersion(mod)}
                  />
                </TooltipTrigger>
                <TooltipContent>
                  <Trans key="instances:_trn_switch_version" />
                </TooltipContent>
              </Tooltip>
            </div>
          </Show>
        )
      },
      enableSorting: false
    })
  ]
}
