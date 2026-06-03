import { Tooltip, TooltipContent, TooltipTrigger, Spinner } from "@gd/ui"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { Show } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { getPlatformKey } from "@gd/i18n/helpers"
import { createColumnHelper } from "@tanstack/solid-table"
import {
  createSelectColumn,
  createFilenameColumn,
  createTypeColumn,
  createEnabledColumn,
  createDeleteColumn
} from "./index"

/**
 * Unified column config for both Instance and Server addon tables.
 * Optional callbacks/fields allow graceful degradation when a feature
 * is unavailable (e.g., server addons have no image metadata yet).
 */
export interface AddonColumnConfig {
  // Required
  selectedCount: () => number
  totalRows: () => number
  onSelectAll: () => void
  onToggleMod: (row: any) => void
  onDeleteMod: (row: any) => void
  getDisplayName: (row: any) => string
  getSubtitle?: (row: any) => string | null
  getAddonType: (row: any) => string

  // Locking (instance-specific, optional)
  isLocked?: () => boolean

  // Icon/thumbnail (optional — needs getImageUrl)
  getImageUrl?: (row: any) => string | null

  // Duplicate warning (optional — needs isDuplicate)
  isDuplicate?: (row: any) => boolean

  // Platform (optional — needs getPlatformInfo)
  getPlatformInfo?: (row: any) => {
    hasCurseforge: boolean
    hasModrinth: boolean
  }

  // Update (optional — needs hasUpdate + onUpdateMod + isModUpdating)
  hasUpdate?: (row: any) => boolean
  onUpdateMod?: (row: any) => void
  isModUpdating?: (rowId: string) => boolean

  // Switch version (optional — needs onSwitchVersion + hasPlatformData)
  onSwitchVersion?: (row: any) => void
}

export const createAddonColumns = (config: AddonColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<any>()

  const lockedTooltip = config.isLocked ? (
    <Trans key="instances:_trn_locked_cannot_apply_changes" />
  ) : undefined

  const columns: any[] = [
    // 1. Selection
    createSelectColumn({
      selectedCount: config.selectedCount,
      totalRows: config.totalRows,
      onSelectAll: config.onSelectAll
    }),

    // 2. Icon / thumbnail
    columnHelper.display({
      id: "icon",
      size: 48,
      header: "",
      cell: (props: any) => {
        const row = props.row.original
        const imageUrl = config.getImageUrl?.(row) ?? null

        return (
          <div class="flex items-center justify-center">
            <Show
              when={imageUrl}
              fallback={
                <div class="bg-darkSlate-600 flex h-8 w-8 items-center justify-center rounded">
                  <div class="i-hugeicons:file-01 text-lightSlate-400 text-lg" />
                </div>
              }
            >
              <div class="bg-darkSlate-600 flex h-8 w-8 items-center justify-center overflow-hidden rounded">
                <img
                  src={imageUrl!}
                  class="h-full w-full object-cover"
                  alt={config.getDisplayName(row)}
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

    // 3. Filename / display name
    createFilenameColumn({
      getDisplayName: config.getDisplayName,
      getSubtitle: config.getSubtitle
    }),

    // 4. Duplicate warning (only renders content when isDuplicate returns true)
    columnHelper.display({
      id: "duplicate",
      header: "",
      size: 48,
      cell: (props: any) => {
        const row = props.row.original
        const isDup = config.isDuplicate?.(row) ?? false
        return (
          <Show when={isDup}>
            <Tooltip>
              <TooltipTrigger>
                <div class="i-hugeicons:alert-01 hidden text-lg text-yellow-500 lg:flex" />
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

    // 5. Type badge
    createTypeColumn({
      getAddonType: config.getAddonType
    }),

    // 6. Platform
    columnHelper.accessor(
      (row: any) => {
        const info = config.getPlatformInfo?.(row)
        if (!info) return "z_local"
        if (info.hasCurseforge && info.hasModrinth) return "b_both"
        if (info.hasCurseforge) return "a_curseforge"
        if (info.hasModrinth) return "c_modrinth"
        return "z_local"
      },
      {
        id: "platform",
        header: () => (
          <span class="hidden md:inline">
            {t("content:_trn_table.platform")}
          </span>
        ),
        size: 98,
        cell: (props: any) => {
          const row = props.row.original
          const info = config.getPlatformInfo?.(row)

          if (!info) {
            // No platform info available (e.g., server addons without metadata)
            return (
              <Tooltip>
                <TooltipTrigger>
                  <div class="i-hugeicons:folder-01 hidden text-lg text-gray-500 md:flex" />
                </TooltipTrigger>
                <TooltipContent>{t("content:_trn_table.local")}</TooltipContent>
              </Tooltip>
            )
          }

          const { hasCurseforge, hasModrinth } = info

          if (!hasCurseforge && !hasModrinth) {
            return (
              <Tooltip>
                <TooltipTrigger>
                  <div class="i-hugeicons:folder-01 hidden text-lg text-gray-500 md:flex" />
                </TooltipTrigger>
                <TooltipContent>{t("content:_trn_table.local")}</TooltipContent>
              </Tooltip>
            )
          }

          if (hasCurseforge && hasModrinth) {
            return (
              <div class="relative hidden h-6 w-6 md:block">
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

          const platform = hasCurseforge ? "curseforge" : "modrinth"
          const logo = hasCurseforge ? CurseforgeLogo : ModrinthLogo
          return (
            <Tooltip>
              <TooltipTrigger>
                <img
                  src={logo}
                  class="hidden h-4 w-4 md:block"
                  alt={t(getPlatformKey(platform))}
                />
              </TooltipTrigger>
              <TooltipContent>{t(getPlatformKey(platform))}</TooltipContent>
            </Tooltip>
          )
        }
      }
    ),

    // 7. Update available
    columnHelper.accessor((row: any) => (config.hasUpdate?.(row) ? 0 : 1), {
      id: "update",
      header: () => t("content:_trn_table.update"),
      size: 100,
      sortingFn: "basic",
      cell: (props: any) => {
        const row = props.row.original
        const hasUpd = config.hasUpdate?.(row) ?? false
        const isUpdating = () => config.isModUpdating?.(row.id) ?? false
        const isDisabled = () => isUpdating() || (config.isLocked?.() ?? false)

        const handleUpdate = () => {
          if (isDisabled()) return
          config.onUpdateMod?.(row)
        }

        return (
          <Show
            when={hasUpd || isUpdating()}
            fallback={<span class="text-lightSlate-600 text-center">-</span>}
          >
            <Tooltip>
              <TooltipTrigger>
                <Show
                  when={!isUpdating()}
                  fallback={<Spinner class="h-5 w-5 text-blue-400" />}
                >
                  <div
                    class="i-hugeicons:download-02 cursor-pointer text-lg transition-colors"
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
                  when={config.isLocked?.()}
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

    // 8. Enabled toggle
    createEnabledColumn({
      onToggle: config.onToggleMod,
      isDisabled: config.isLocked,
      disabledTooltip: lockedTooltip
    }),

    // 9. Switch version
    columnHelper.display({
      id: "actions",
      header: () => (
        <span class="hidden lg:inline">{t("content:_trn_table.actions")}</span>
      ),
      size: 80,
      cell: (props: any) => {
        const row = props.row.original
        const info = config.getPlatformInfo?.(row)
        const hasPlatformData = info
          ? info.hasCurseforge || info.hasModrinth
          : false

        return (
          <Show when={hasPlatformData && config.onSwitchVersion}>
            <div
              class="hidden items-center justify-center lg:flex"
              onMouseDown={(e) => e.stopPropagation()}
            >
              <Tooltip>
                <TooltipTrigger>
                  <div
                    class="i-hugeicons:arrow-left-right text-lightSlate-400 hover:text-lightSlate-200 h-5 w-5 cursor-pointer transition-colors"
                    onClick={() => config.onSwitchVersion!(row)}
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
    }),

    // 10. Delete
    createDeleteColumn({
      onDelete: config.onDeleteMod,
      isDisabled: config.isLocked,
      disabledTooltip: lockedTooltip
    })
  ]

  return columns
}
