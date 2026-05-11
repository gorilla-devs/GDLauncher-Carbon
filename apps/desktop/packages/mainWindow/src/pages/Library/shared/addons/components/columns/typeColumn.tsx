import { Badge } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { getAddonTabKey } from "@gd/i18n/helpers"
import { createColumnHelper } from "@tanstack/solid-table"
import { getAddonTypeIcon } from "@/utils/addonIcons"

interface TypeColumnConfig {
  getAddonType: (row: any) => string
}

export const createTypeColumn = (config: TypeColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<any>()

  return columnHelper.accessor((row) => config.getAddonType(row), {
    id: "type",
    header: () => (
      <span class="hidden lg:inline">{t("content:_trn_table.type")}</span>
    ),
    size: 124,
    cell: (props) => {
      const addonType = config.getAddonType(props.row.original)
      return (
        <div class="hidden lg:flex">
          <Badge variant="secondary" class="flex items-center gap-1.5">
            <div
              class={`${getAddonTypeIcon(addonType as any)} shrink-0 text-base`}
            />
            {t(getAddonTabKey(addonType as any))}
          </Badge>
        </div>
      )
    },
    sortingFn: "alphanumeric"
  })
}
