import {
  Badge,
  Button,
  Checkbox,
  Switch,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { Show } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { createColumnHelper } from "@tanstack/solid-table"
import { Mod as ModType } from "@gd/core_module/bindings"

interface ColumnConfig {
  isInstanceLocked: () => boolean
  selectedCount: () => number
  totalRows: () => number
  onSelectAll: () => void
  onToggleMod: (mod: ModType) => Promise<void>
  onUpdateMod: (mod: ModType) => Promise<void>
  onDeleteMod: (mod: ModType) => Promise<void>
}

export const createAddonColumns = (config: ColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<ModType>()

  return [
    // Selection column
    columnHelper.display({
      id: "select",
      size: 40,
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
            onChange={(checked) => {
              console.log("Header checkbox onChange:", checked)
              config.onSelectAll()
            }}
          />
        )
      },
      cell: (props) => (
        <div
          class={`transition-opacity duration-100 ease-in-out ${
            props.row.getIsSelected()
              ? "opacity-100"
              : "opacity-0 group-hover:opacity-100"
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

    // Filename column
    columnHelper.accessor("filename", {
      header: t("instance.table.filename"),
      size: 400,
      cell: (info) => {
        const mod = info.row.original
        const displayName = mod.metadata?.name || mod.filename
        return (
          <div class="flex items-center gap-2">
            <div class="flex flex-col">
              <span class="font-medium">{displayName}</span>
              <Show when={mod.metadata?.name}>
                <span class="text-sm text-lightSlate-600">{mod.filename}</span>
              </Show>
            </div>
          </div>
        )
      }
    }),

    // Type column
    columnHelper.display({
      id: "type",
      header: t("instance.table.type"),
      size: 100,
      cell: (props) => {
        const mod = props.row.original
        return (
          <Badge variant="secondary">
            {t(`instance.tabs.${mod.addon_type}`)}
          </Badge>
        )
      }
    }),

    // Platform column
    columnHelper.display({
      id: "platform",
      header: t("instance.table.platform"),
      size: 80,
      cell: (props) => {
        const mod = props.row.original
        const platforms = []

        if (mod.curseforge) {
          platforms.push(
            <Tooltip>
              <TooltipTrigger>
                <img src={CurseforgeLogo} class="h-4 w-4" alt="CurseForge" />
              </TooltipTrigger>
              <TooltipContent>CurseForge</TooltipContent>
            </Tooltip>
          )
        }
        if (mod.modrinth) {
          platforms.push(
            <Tooltip>
              <TooltipTrigger>
                <img src={ModrinthLogo} class="h-4 w-4" alt="Modrinth" />
              </TooltipTrigger>
              <TooltipContent>Modrinth</TooltipContent>
            </Tooltip>
          )
        }

        // If no platforms, show local
        if (platforms.length === 0) {
          platforms.push(
            <Tooltip>
              <TooltipTrigger>
                <div class="i-ri:folder-fill text-lg text-gray-500" />
              </TooltipTrigger>
              <TooltipContent>{t("instance.table.local")}</TooltipContent>
            </Tooltip>
          )
        }

        return <div class="flex flex-wrap gap-1">{platforms}</div>
      }
    }),

    // Status/Enable column
    columnHelper.accessor("enabled", {
      header: t("instance.table.status"),
      size: 80,
      cell: (props) => {
        const mod = props.row.original
        return (
          <Show
            when={!config.isInstanceLocked()}
            fallback={
              <Tooltip>
                <TooltipTrigger>
                  <Switch checked={props.getValue()} disabled />
                </TooltipTrigger>
                <TooltipContent>
                  <Trans key="instance.locked_cannot_apply_changes" />
                </TooltipContent>
              </Tooltip>
            }
          >
            <Switch
              checked={mod.enabled}
              onChange={() => config.onToggleMod(mod)}
            />
          </Show>
        )
      }
    }),

    // Actions column
    columnHelper.display({
      id: "actions",
      header: t("instance.table.actions"),
      size: 100,
      cell: (props) => {
        const mod = props.row.original
        return (
          <div class="flex items-center gap-2">
            <Show when={mod.has_update}>
              <Tooltip>
                <TooltipTrigger>
                  <Button
                    size="small"
                    disabled={config.isInstanceLocked()}
                    onClick={() => config.onUpdateMod(mod)}
                  >
                    <div class="i-ri:download-2-fill" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  {config.isInstanceLocked()
                    ? t("instance.locked_cannot_apply_changes")
                    : t("instance.update_mod")}
                </TooltipContent>
              </Tooltip>
            </Show>
            <Tooltip>
              <TooltipTrigger>
                <Button
                  size="small"
                  disabled={config.isInstanceLocked()}
                  onClick={() => config.onDeleteMod(mod)}
                >
                  <div class="i-ri:delete-bin-2-fill" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {config.isInstanceLocked()
                  ? t("instance.locked_cannot_apply_changes")
                  : t("instance.delete_mod")}
              </TooltipContent>
            </Tooltip>
          </div>
        )
      },
      enableSorting: false
    })
  ]
}
