import {
  For,
  Show,
  onMount,
  createSignal,
  onCleanup,
  createMemo
} from "solid-js"
import {
  flexRender,
  getCoreRowModel,
  createSolidTable,
  getSortedRowModel,
  getFilteredRowModel,
  SortingState,
  ColumnFiltersState,
  VisibilityState,
  ColumnDef,
  RowSelectionState
} from "@tanstack/solid-table"
import {
  ContextMenu,
  ContextMenuTrigger,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator
} from "@gd/ui"
import { Mod as ModType } from "@gd/core_module/bindings"

interface AddonTableProps {
  data: () => ModType[]
  columns: ColumnDef<ModType, any>[]
  sorting: () => SortingState
  setSorting: (sorting: SortingState) => void
  columnFilters: () => ColumnFiltersState
  setColumnFilters: (filters: ColumnFiltersState) => void
  columnVisibility: () => VisibilityState
  setColumnVisibility: (visibility: VisibilityState) => void
  rowSelection: () => RowSelectionState
  setRowSelection: (
    selection:
      | RowSelectionState
      | ((prev: RowSelectionState) => RowSelectionState)
  ) => void
  onTableReady?: (table: any) => void
  hasBulkActions?: boolean
}

export const AddonTable = (props: AddonTableProps) => {
  const [scrollTop, setScrollTop] = createSignal(0)
  const [containerHeight, setContainerHeight] = createSignal(800)
  let tableRef: HTMLDivElement | undefined

  // Drag selection state
  const [isDragging, setIsDragging] = createSignal(false)
  const [dragStartRow, setDragStartRow] = createSignal<string | null>(null)
  const [dragMode, setDragMode] = createSignal<"select" | "deselect">("select")
  const [previewSelection, setPreviewSelection] = createSignal<Set<string>>(
    new Set()
  )

  // Context menu state
  const [contextMenuSelection, setContextMenuSelection] = createSignal<
    Set<string>
  >(new Set())
  const [isContextMenuOpen, setIsContextMenuOpen] = createSignal(false)
  const [shouldShowContextMenu, setShouldShowContextMenu] = createSignal(false)

  const table = createSolidTable({
    get data() {
      return props.data()
    },
    columns: props.columns,
    state: {
      get sorting() {
        return props.sorting()
      },
      get columnFilters() {
        return props.columnFilters()
      },
      get columnVisibility() {
        return props.columnVisibility()
      },
      get rowSelection() {
        return props.rowSelection()
      }
    },
    onSortingChange: (updater) => {
      if (typeof updater === "function") {
        props.setSorting(updater(props.sorting()))
      } else {
        props.setSorting(updater)
      }
    },
    onColumnFiltersChange: (updater) => {
      if (typeof updater === "function") {
        props.setColumnFilters(updater(props.columnFilters()))
      } else {
        props.setColumnFilters(updater)
      }
    },
    onColumnVisibilityChange: (updater) => {
      if (typeof updater === "function") {
        props.setColumnVisibility(updater(props.columnVisibility()))
      } else {
        props.setColumnVisibility(updater)
      }
    },
    onRowSelectionChange: (updater) => {
      if (typeof updater === "function") {
        props.setRowSelection(updater)
      } else {
        props.setRowSelection(updater)
      }
    },
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getRowId: (row) => row.id
  })

  // Virtual scrolling calculations
  const ROW_HEIGHT = 60
  const BUFFER_SIZE = 5

  const rows = createMemo(() => table.getRowModel().rows)

  // Drag selection utilities
  const getRowRange = (startRowId: string, endRowId: string) => {
    const allRows = rows()
    const startIndex = allRows.findIndex((row) => row.id === startRowId)
    const endIndex = allRows.findIndex((row) => row.id === endRowId)

    if (startIndex === -1 || endIndex === -1) return []

    const [minIndex, maxIndex] = [
      Math.min(startIndex, endIndex),
      Math.max(startIndex, endIndex)
    ]
    return allRows.slice(minIndex, maxIndex + 1).map((row) => row.id)
  }

  const updatePreviewSelection = (currentRowId: string) => {
    const startRowId = dragStartRow()
    if (!startRowId) return

    const rowIds = getRowRange(startRowId, currentRowId)
    setPreviewSelection(new Set(rowIds))
  }

  const visibleRows = createMemo(() => {
    const allRows = rows()
    const total = allRows.length
    if (total === 0) return { start: 0, end: 0, total: 0, rows: [] }

    // Use scrollTop signal for reactivity
    const currentScrollTop = scrollTop()

    const start = Math.max(
      0,
      Math.floor(currentScrollTop / ROW_HEIGHT) - BUFFER_SIZE
    )
    const end = Math.min(
      total,
      Math.ceil((currentScrollTop + containerHeight()) / ROW_HEIGHT) +
        BUFFER_SIZE
    )

    console.log("Virtualization update:", {
      scrollTop: currentScrollTop,
      containerHeight: containerHeight(),
      start,
      end,
      total,
      visibleCount: end - start
    })

    return {
      start,
      end,
      total,
      rows: allRows.slice(start, end)
    }
  })

  // Drag selection event handlers
  const handleMouseDown = (rowId: string, event: MouseEvent) => {
    // Don't start drag selection if this is a right-click on a selected row
    if (event.button === 2) {
      const selectedRowIds = Object.keys(props.rowSelection()).filter(
        (id) => props.rowSelection()[id]
      )
      if (selectedRowIds.includes(rowId)) {
        // This is a right-click on a selected row, don't interfere
        return
      }
    }

    event.preventDefault()

    const currentRowSelection = props.rowSelection()
    const isCurrentlySelected = currentRowSelection[rowId]

    setIsDragging(true)
    setDragStartRow(rowId)
    setDragMode(isCurrentlySelected ? "deselect" : "select")
    setPreviewSelection(new Set([rowId]))

    document.body.style.userSelect = "none"
    document.body.style.cursor = "grabbing"
  }

  const handleMouseEnter = (rowId: string) => {
    if (!isDragging()) return

    updatePreviewSelection(rowId)
  }

  const handleMouseUp = () => {
    if (!isDragging()) return

    const preview = previewSelection()
    const mode = dragMode()

    if (preview.size > 0) {
      const currentSelection = props.rowSelection()
      const newSelection = { ...currentSelection }

      preview.forEach((rowId) => {
        newSelection[rowId] = mode === "select"
      })

      props.setRowSelection(newSelection)
    }

    setIsDragging(false)
    setDragStartRow(null)
    setPreviewSelection(new Set<string>())

    document.body.style.userSelect = ""
    document.body.style.cursor = ""
  }

  // Context menu event handlers
  const handleContextMenu = (rowId: string, _event: MouseEvent) => {
    const selectedRowIds = Object.keys(props.rowSelection()).filter(
      (id) => props.rowSelection()[id]
    )

    // Check if the clicked row is selected
    if (!selectedRowIds.includes(rowId)) {
      // Auto-select the right-clicked row
      const currentSelection = props.rowSelection()
      const newSelection = { ...currentSelection, [rowId]: true }
      props.setRowSelection(newSelection)

      // Update context menu to include the newly selected row
      const updatedSelectedIds = [...selectedRowIds, rowId]
      setContextMenuSelection(new Set(updatedSelectedIds))
    } else {
      // Row was already selected, use existing selection
      setContextMenuSelection(new Set(selectedRowIds))
    }

    setShouldShowContextMenu(true)
    return true
  }

  const handleContextMenuOpenChange = (open: boolean) => {
    setIsContextMenuOpen(open)
    if (!open) {
      setContextMenuSelection(new Set<string>())
      setShouldShowContextMenu(false)
    }
  }

  const getContextMenuItems = () => {
    const selectedCount = contextMenuSelection().size

    return [
      {
        type: "item",
        label: `Selected ${selectedCount} item${selectedCount !== 1 ? "s" : ""}`,
        disabled: true,
        id: "header"
      },
      { type: "separator" },
      {
        type: "item",
        label: "Test Action 1",
        action: () => console.log("Action 1 on", contextMenuSelection()),
        id: "action1"
      },
      {
        type: "item",
        label: "Test Action 2",
        action: () => console.log("Action 2 on", contextMenuSelection()),
        id: "action2"
      },
      { type: "separator" },
      {
        type: "item",
        label: "Delete Selected",
        action: () => console.log("Delete", contextMenuSelection()),
        destructive: true,
        id: "delete"
      }
    ]
  }

  const getRowClasses = (rowId: string) => {
    const baseClasses =
      "border-darkSlate-600 hover:bg-darkSlate-750 flex w-full border-t group cursor-pointer"

    // Context menu highlighting (highest priority)
    if (contextMenuSelection().has(rowId) && isContextMenuOpen()) {
      return `${baseClasses} bg-blue-500/10 ring-1 ring-blue-400/30`
    }

    // Drag preview highlighting
    const preview = previewSelection()
    if (preview.has(rowId)) {
      const mode = dragMode()
      return `${baseClasses} ${
        mode === "select"
          ? "bg-blue-500/10 ring-1 ring-blue-400/30"
          : "bg-red-500/10 ring-1 ring-red-400/30"
      }`
    }

    return baseClasses
  }

  // Handle scroll events for virtual scrolling
  onMount(() => {
    if (props.onTableReady) {
      props.onTableReady(table)
    }

    // Wait a tick for DOM to be ready
    setTimeout(() => {
      const scrollContainer = document.getElementById(
        "main-container-instance-details"
      )

      if (!scrollContainer) {
        console.error("Could not find main-container-instance-details")
        return
      }

      const handleScroll = () => {
        if (!tableRef) return

        requestAnimationFrame(() => {
          // Get the table position relative to the scrollable container
          const tableRect = tableRef.getBoundingClientRect()
          const containerRect = scrollContainer.getBoundingClientRect()

          // Calculate how much the table has scrolled past the top of the container
          const tableScrollOffset = containerRect.top - tableRect.top
          const newScrollTop = Math.max(0, tableScrollOffset)

          setScrollTop(newScrollTop)
        })
      }

      const updateHeight = () => {
        setContainerHeight(window.innerHeight)
      }

      updateHeight()
      handleScroll() // Initial check

      scrollContainer.addEventListener("scroll", handleScroll, {
        passive: true
      })
      window.addEventListener("resize", updateHeight)

      onCleanup(() => {
        scrollContainer.removeEventListener("scroll", handleScroll)
        window.removeEventListener("resize", updateHeight)
      })
    }, 100)

    // Global mouse event listeners for drag selection
    document.addEventListener("mouseup", handleMouseUp)

    onCleanup(() => {
      document.removeEventListener("mouseup", handleMouseUp)
      // Reset body styles if component unmounts during drag
      document.body.style.userSelect = ""
      document.body.style.cursor = ""
    })
  })

  return (
    <ContextMenu onOpenChange={handleContextMenuOpenChange}>
      <ContextMenuTrigger class="border-darkSlate-600 rounded-lg border">
        {/* Sticky Table Header - sticks below filters and optionally bulk actions */}
        <div
          class="bg-darkSlate-700 sticky z-10 rounded-t-lg"
          style={{ top: props.hasBulkActions ? "189px" : "115px" }}
        >
          <For each={table.getHeaderGroups()}>
            {(headerGroup) => (
              <div class="flex">
                <For each={headerGroup.headers}>
                  {(header) => (
                    <div
                      class="text-lightSlate-300 min-w-0 flex-1 px-4 py-3 text-left text-sm font-medium"
                      style={{ width: `${header.getSize()}px` }}
                    >
                      <Show when={!header.isPlaceholder} fallback={null}>
                        <div
                          class={`flex items-center gap-2 ${
                            header.column.getCanSort()
                              ? "cursor-pointer select-none"
                              : ""
                          }`}
                          onClick={header.column.getToggleSortingHandler()}
                        >
                          {flexRender(
                            header.column.columnDef.header,
                            header.getContext()
                          )}
                          <Show when={header.column.getCanSort()}>
                            <div class="flex flex-col">
                              <div
                                class={`i-ri:arrow-up-s-line text-xs ${
                                  header.column.getIsSorted() === "asc"
                                    ? "text-blue-400"
                                    : "text-lightSlate-500"
                                }`}
                              />
                              <div
                                class={`i-ri:arrow-down-s-line -mt-1 text-xs ${
                                  header.column.getIsSorted() === "desc"
                                    ? "text-blue-400"
                                    : "text-lightSlate-500"
                                }`}
                              />
                            </div>
                          </Show>
                        </div>
                      </Show>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </div>

        {/* Table Body */}
        <div ref={tableRef}>
          {/* Virtual spacer before visible items */}
          <div style={{ height: `${visibleRows().start * ROW_HEIGHT}px` }} />

          {/* Only render visible rows */}
          <For each={visibleRows().rows}>
            {(row) => (
              <div
                class={getRowClasses(row.id)}
                style={{ height: `${ROW_HEIGHT}px` }}
                data-row-id={row.id}
                onMouseDown={(e) => handleMouseDown(row.id, e)}
                onMouseEnter={() => handleMouseEnter(row.id)}
                onContextMenu={(e) => handleContextMenu(row.id, e)}
              >
                <For each={row.getVisibleCells()}>
                  {(cell) => (
                    <div
                      class="flex min-w-0 flex-1 items-center px-4 py-3 text-sm"
                      style={{ width: `${cell.column.getSize()}px` }}
                    >
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext()
                      )}
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>

          {/* Virtual spacer after visible items */}
          <div
            style={{
              height: `${(visibleRows().total - visibleRows().end) * ROW_HEIGHT}px`
            }}
          />
        </div>
      </ContextMenuTrigger>

      <ContextMenuContent>
        <Show when={shouldShowContextMenu()}>
          <For each={getContextMenuItems()}>
            {(item) => (
              <Show
                when={item.type === "separator"}
                fallback={
                  <ContextMenuItem
                    disabled={item.disabled}
                    class={
                      item.destructive ? "text-red-400 focus:text-red-300" : ""
                    }
                    onSelect={item.action}
                  >
                    {item.label}
                  </ContextMenuItem>
                }
              >
                <ContextMenuSeparator />
              </Show>
            )}
          </For>
        </Show>
      </ContextMenuContent>
    </ContextMenu>
  )
}
