import {
  For,
  Show,
  onMount,
  createSignal,
  onCleanup,
  createMemo,
  createEffect,
  Accessor
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
  RowSelectionState,
  Table
} from "@tanstack/solid-table"
import {
  ContextMenu,
  ContextMenuTrigger,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator
} from "@gd/ui"
import { Mod as ModType } from "@gd/core_module/bindings"

interface VirtualizationConfig {
  /** Fixed row height or function to get height for each row index */
  rowHeight?: number | ((index: number) => number)
  /** Number of rows to render outside visible area for smoother scrolling */
  bufferSize?: number
  /** Enable dynamic height measurement for rows */
  enableDynamicHeight?: boolean
}

interface AddonTableProps {
  data: Accessor<ModType[]>
  columns: ColumnDef<ModType, any>[]
  sorting: Accessor<SortingState>
  setSorting: (sorting: SortingState) => void
  columnFilters: Accessor<ColumnFiltersState>
  setColumnFilters: (filters: ColumnFiltersState) => void
  columnVisibility: Accessor<VisibilityState>
  setColumnVisibility: (visibility: VisibilityState) => void
  rowSelection: Accessor<RowSelectionState>
  setRowSelection: (
    selection:
      | RowSelectionState
      | ((prev: RowSelectionState) => RowSelectionState)
  ) => void
  onTableReady?: (table: Table<ModType>) => void
  hasBulkActions?: boolean
  /** Optional reference to scroll container, defaults to finding closest scrollable parent */
  scrollContainerRef?: HTMLElement
  /** Configuration for virtualization behavior */
  virtualizationConfig?: VirtualizationConfig
}

function throttle<T extends (...args: any[]) => any>(
  func: T,
  delay: number
): (...args: Parameters<T>) => void {
  let lastCall = 0
  let timeout: number | null = null

  return (...args: Parameters<T>) => {
    const now = Date.now()

    if (now - lastCall >= delay) {
      lastCall = now
      func(...args)
    } else if (!timeout) {
      timeout = window.setTimeout(
        () => {
          lastCall = Date.now()
          func(...args)
          timeout = null
        },
        delay - (now - lastCall)
      )
    }
  }
}

class RowHeightCache {
  private cache = new Map<number, number>()
  private defaultHeight: number

  constructor(defaultHeight: number) {
    this.defaultHeight = defaultHeight
  }

  get(index: number): number {
    return this.cache.get(index) ?? this.defaultHeight
  }

  set(index: number, height: number): void {
    this.cache.set(index, height)
  }

  clear(): void {
    this.cache.clear()
  }

  getOffsetForIndex(index: number): number {
    let offset = 0
    for (let i = 0; i < index; i++) {
      offset += this.get(i)
    }
    return offset
  }

  getTotalHeight(totalRows: number): number {
    let height = 0
    for (let i = 0; i < totalRows; i++) {
      height += this.get(i)
    }
    return height
  }
}

export const AddonTable = (props: AddonTableProps) => {
  const [scrollTop, setScrollTop] = createSignal(0)
  const [containerHeight, setContainerHeight] = createSignal(window.innerHeight)
  let tableRef: HTMLDivElement | undefined
  let resizeObserver: ResizeObserver | undefined
  let scrollHandlerCleanup: (() => void) | undefined

  const config = props.virtualizationConfig ?? {}
  const defaultRowHeight =
    typeof config.rowHeight === "number" ? config.rowHeight : 60
  const rowHeightCache = new RowHeightCache(defaultRowHeight)

  const [isDragging, setIsDragging] = createSignal(false)
  const [dragStartRow, setDragStartRow] = createSignal<string | null>(null)
  const [dragMode, setDragMode] = createSignal<"select" | "deselect">("select")
  const [previewSelection, setPreviewSelection] = createSignal<Set<string>>(
    new Set()
  )

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

  const BUFFER_SIZE = config.bufferSize ?? 5
  const enableDynamicHeight = config.enableDynamicHeight ?? false

  const getRowHeight = (index: number): number => {
    if (typeof config.rowHeight === "function") {
      return config.rowHeight(index)
    }
    return rowHeightCache.get(index)
  }

  const rows = createMemo(() => table.getRowModel().rows)

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
    if (total === 0)
      return {
        start: 0,
        end: 0,
        total: 0,
        rows: [],
        startOffset: 0,
        endOffset: 0
      }

    // Use scrollTop signal for reactivity
    const currentScrollTop = scrollTop()
    const currentContainerHeight = containerHeight()

    if (enableDynamicHeight) {
      // Dynamic height calculation
      let accumulatedHeight = 0
      let start = 0
      let end = total

      // Find start index
      for (let i = 0; i < total; i++) {
        if (
          accumulatedHeight >=
          currentScrollTop - BUFFER_SIZE * defaultRowHeight
        ) {
          start = Math.max(0, i - BUFFER_SIZE)
          break
        }
        accumulatedHeight += getRowHeight(i)
      }

      // Find end index
      accumulatedHeight = rowHeightCache.getOffsetForIndex(start)
      for (let i = start; i < total; i++) {
        if (
          accumulatedHeight >
          currentScrollTop +
            currentContainerHeight +
            BUFFER_SIZE * defaultRowHeight
        ) {
          end = Math.min(total, i + BUFFER_SIZE)
          break
        }
        accumulatedHeight += getRowHeight(i)
      }

      return {
        start,
        end,
        total,
        rows: allRows.slice(start, end),
        startOffset: rowHeightCache.getOffsetForIndex(start),
        endOffset:
          rowHeightCache.getTotalHeight(total) -
          rowHeightCache.getOffsetForIndex(end)
      }
    } else {
      // Fixed height calculation (optimized)
      const fixedHeight = defaultRowHeight
      const start = Math.max(
        0,
        Math.floor(currentScrollTop / fixedHeight) - BUFFER_SIZE
      )
      const end = Math.min(
        total,
        Math.ceil((currentScrollTop + currentContainerHeight) / fixedHeight) +
          BUFFER_SIZE
      )

      return {
        start,
        end,
        total,
        rows: allRows.slice(start, end),
        startOffset: start * fixedHeight,
        endOffset: (total - end) * fixedHeight
      }
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

    if (contextMenuSelection().has(rowId) && isContextMenuOpen()) {
      return `${baseClasses} bg-blue-500/10 ring-1 ring-blue-400/30`
    }

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

  const findScrollContainer = (): HTMLElement | null => {
    if (props.scrollContainerRef) return props.scrollContainerRef

    const byId = document.getElementById("main-container-instance-details")
    if (byId) return byId

    let parent = tableRef?.parentElement
    while (parent) {
      const style = window.getComputedStyle(parent)
      if (
        style.overflow === "auto" ||
        style.overflow === "scroll" ||
        style.overflowY === "auto" ||
        style.overflowY === "scroll"
      ) {
        return parent
      }
      parent = parent.parentElement
    }

    return document.documentElement
  }

  const createScrollHandler = (container: HTMLElement) => {
    let ticking = false
    let cachedTableRect: DOMRect | null = null
    let cacheTimeout: number | null = null

    const updateCache = () => {
      cachedTableRect = tableRef?.getBoundingClientRect() ?? null
      cacheTimeout = window.setTimeout(() => {
        cachedTableRect = null
      }, 100)
    }

    const handleScroll = () => {
      if (ticking || !tableRef) return

      ticking = true
      requestAnimationFrame(() => {
        try {
          if (!cachedTableRect) updateCache()
          if (!cachedTableRect) return

          const containerRect = container.getBoundingClientRect()
          const tableScrollOffset = containerRect.top - cachedTableRect.top
          const newScrollTop = Math.max(0, tableScrollOffset)

          setScrollTop(newScrollTop)
        } finally {
          ticking = false
        }
      })
    }

    const throttledHandler = throttle(handleScroll, 16)

    return {
      handler: throttledHandler,
      cleanup: () => {
        if (cacheTimeout) clearTimeout(cacheTimeout)
      }
    }
  }

  onMount(() => {
    if (props.onTableReady) {
      props.onTableReady(table)
    }

    createEffect(() => {
      scrollHandlerCleanup?.()
      resizeObserver?.disconnect()

      const container = findScrollContainer()
      if (!container) {
        console.warn("AddonTable: Could not find scroll container")
        return
      }

      const { handler, cleanup } = createScrollHandler(container)

      resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          if (entry.target === container) {
            setContainerHeight(entry.contentRect.height)
          }
        }
      })

      resizeObserver.observe(container)

      setContainerHeight(container.clientHeight)
      handler()

      container.addEventListener("scroll", handler, { passive: true })

      scrollHandlerCleanup = () => {
        container.removeEventListener("scroll", handler)
        cleanup()
      }
    })

    document.addEventListener("mouseup", handleMouseUp)

    onCleanup(() => {
      scrollHandlerCleanup?.()
      resizeObserver?.disconnect()
      document.removeEventListener("mouseup", handleMouseUp)
      document.body.style.userSelect = ""
      document.body.style.cursor = ""
    })
  })

  return (
    <ContextMenu onOpenChange={handleContextMenuOpenChange}>
      <ContextMenuTrigger class="border-darkSlate-600 rounded-lg border">
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

        <div ref={tableRef}>
          <div
            style={{
              height: `${visibleRows().startOffset}px`,
              "will-change": "height"
            }}
          />

          <For each={visibleRows().rows}>
            {(row, index) => {
              const rowIndex = () => visibleRows().start + index()
              const rowHeight = () => getRowHeight(rowIndex())

              return (
                <div
                  ref={(el) => {
                    if (enableDynamicHeight && el) {
                      const observer = new ResizeObserver((entries) => {
                        const entry = entries[0]
                        if (entry) {
                          const height = entry.contentRect.height
                          if (height > 0) {
                            rowHeightCache.set(rowIndex(), height)
                          }
                        }
                      })
                      observer.observe(el)
                      onCleanup(() => observer.disconnect())
                    }
                  }}
                  class={getRowClasses(row.id)}
                  style={{
                    height: enableDynamicHeight ? "auto" : `${rowHeight()}px`,
                    "min-height": enableDynamicHeight
                      ? `${defaultRowHeight}px`
                      : undefined,
                    "will-change": "transform"
                  }}
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
              )
            }}
          </For>

          <div
            style={{
              height: `${visibleRows().endOffset}px`,
              "will-change": "height"
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
