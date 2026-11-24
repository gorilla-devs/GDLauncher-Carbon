import { Checkbox } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { createColumnHelper } from "@tanstack/solid-table"

interface SelectColumnConfig {
  selectedCount: () => number
  totalRows: () => number
  onSelectAll: () => void
  /** Names a row's checkbox after its addon. */
  getDisplayName: (row: any) => string
}

export const createSelectColumn = (config: SelectColumnConfig) => {
  const [t] = useTransContext()
  const columnHelper = createColumnHelper<any>()

  return columnHelper.display({
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
          aria-label={t("content:_trn_a11y_select_all")}
          onChange={() => config.onSelectAll()}
        />
      )
    },
    cell: (props) => (
      <div
        // Shown on hover, and while the keyboard is on it.
        class={`ease-spring transition-opacity duration-100 has-[:focus-visible]:opacity-100 ${
          props.row.getIsSelected()
            ? "opacity-100"
            : "opacity-0 group-hover/row:opacity-100"
        }`}
        onMouseDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
      >
        <Checkbox
          checked={props.row.getIsSelected()}
          disabled={!props.row.getCanSelect()}
          indeterminate={props.row.getIsSomeSelected()}
          aria-label={t("content:_trn_a11y_select_addon", {
            name: config.getDisplayName(props.row.original),
            // A name, not HTML: "Xaero's Minimap" stays as it is.
            interpolation: { escapeValue: false }
          })}
          onChange={(value) => props.row.toggleSelected(!!value)}
        />
      </div>
    ),
    enableSorting: false,
    enableHiding: false
  })
}
