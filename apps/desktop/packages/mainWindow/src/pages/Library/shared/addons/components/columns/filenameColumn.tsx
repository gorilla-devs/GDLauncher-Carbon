import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { Show, createSignal } from "solid-js"
import { useTransContext } from "@gd/i18n"
import { createColumnHelper } from "@tanstack/solid-table"
import CopyIcon from "@/components/CopyIcon"

interface FilenameColumnConfig {
  getDisplayName: (row: any) => string
  getSubtitle?: (row: any) => string | null
}

export const createFilenameColumn = (config: FilenameColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<any>()

  return columnHelper.accessor("filename", {
    header: t("content:_trn_table.filename"),
    sortingFn: (rowA, rowB) => {
      const a = config.getDisplayName(rowA.original)
      const b = config.getDisplayName(rowB.original)
      return a.localeCompare(b, undefined, {
        sensitivity: "base",
        numeric: true
      })
    },
    cell: (info) => {
      const row = info.row.original
      const displayName = config.getDisplayName(row)
      const subtitle = config.getSubtitle?.(row) ?? null
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
                    class="shrink-0 transition-opacity duration-200"
                    classList={{
                      "opacity-0 invisible": !showCopy(),
                      "opacity-100 visible": showCopy()
                    }}
                    onMouseDown={(e) => e.stopPropagation()}
                  >
                    <CopyIcon text={displayName} />
                  </div>
                </div>
                <Show when={subtitle}>
                  <div class="flex min-w-0 items-center gap-2">
                    <div class="text-lightSlate-600 min-w-0 flex-1 truncate text-left text-sm">
                      {subtitle}
                    </div>
                    <div class="w-4 shrink-0" />
                  </div>
                </Show>
              </div>
            </TooltipTrigger>
            <TooltipContent>
              <div class="flex flex-col gap-1">
                <div class="font-medium">{displayName}</div>
                <Show when={subtitle}>
                  <div class="text-lightSlate-400 text-xs">{subtitle}</div>
                </Show>
              </div>
            </TooltipContent>
          </Tooltip>
        </div>
      )
    }
  })
}
