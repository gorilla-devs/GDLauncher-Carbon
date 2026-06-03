import { Checkbox } from "@gd/ui"
import { createColumnHelper } from "@tanstack/solid-table"

interface SelectColumnConfig {
  selectedCount: () => number
  totalRows: () => number
  onSelectAll: () => void
}

export const createSelectColumn = (config: SelectColumnConfig) => {
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
          onChange={() => config.onSelectAll()}
        />
      )
    },
    cell: (props) => (
      <div
        class={`ease-spring transition-opacity duration-100 ${
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
  })
}
