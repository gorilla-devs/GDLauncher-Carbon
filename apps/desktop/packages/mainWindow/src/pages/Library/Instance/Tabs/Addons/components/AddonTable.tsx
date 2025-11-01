import {
  For,
  Show,
  onMount,
  createSignal,
  onCleanup,
  createMemo,
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
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubTrigger,
  ContextMenuSubContent,
  ContextMenuPortal
} from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { Mod as ModType } from "@gd/core_module/bindings"
import { toast } from "@gd/ui"
import { useModal } from "@/managers/ModalsManager"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"

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
  /** Optional reference to scroll container, defaults to finding closest scrollable parent */
  scrollContainerRef?: HTMLElement
  /** Configuration for virtualization behavior */
  virtualizationConfig?: VirtualizationConfig
  isInstanceLocked: () => boolean
  /** Dynamic top offset for sticky header based on filter height */
  headerTopOffset?: number
  /** Mutation handlers */
  mutations?: {
    handleToggleMod: (mod: ModType) => Promise<void>
    handleUpdateMod: (mod: ModType) => Promise<void>
    handleDeleteMod: (mod: ModType) => Promise<void>
    handleDeleteSelected: (mods: ModType[]) => Promise<void>
    handleUpdateSelected: (mods: ModType[]) => Promise<void>
    handleOpenFolder: () => void
    instanceId: number
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
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [scrollTop, setScrollTop] = createSignal(0)
  const [containerHeight, setContainerHeight] = createSignal(window.innerHeight)
  let tableRef: HTMLDivElement | undefined
  let headerRef: HTMLDivElement | undefined
  let resizeObserver: ResizeObserver | undefined
  let scrollHandlerCleanup: (() => void) | undefined
  let autoScrollInterval: number | null = null
  let scrollContainerRef: HTMLElement | null = null

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

  // Handle mouse leave to continue selection outside viewport
  const handleMouseLeave = (event: MouseEvent) => {
    if (!isDragging()) return

    // Continue tracking mouse position for selection updates
    handleMouseMove(event)
  }

  // Auto-scroll when dragging near viewport edges
  const handleMouseMove = (event: MouseEvent) => {
    if (!isDragging() || !scrollContainerRef) return

    const EDGE_THRESHOLD = 80 // Distance from edge to start scrolling
    const SCROLL_SPEED = 15 // Max pixels per frame
    const HEADER_OFFSET = 115 // Account for sticky header

    const containerRect = scrollContainerRef.getBoundingClientRect()
    const mouseY = event.clientY

    // Clear existing interval
    if (autoScrollInterval) {
      clearInterval(autoScrollInterval)
      autoScrollInterval = null
    }

    // Calculate effective boundaries accounting for sticky header
    const effectiveTop = Math.max(containerRect.top, HEADER_OFFSET)
    const effectiveBottom = containerRect.bottom

    // Check if near top edge or above container
    if (mouseY < effectiveTop + EDGE_THRESHOLD) {
      const distanceFromTop = Math.max(0, mouseY - effectiveTop)
      const intensity =
        1 - Math.max(0, Math.min(1, distanceFromTop / EDGE_THRESHOLD))

      autoScrollInterval = window.setInterval(() => {
        if (scrollContainerRef) {
          const currentScroll = scrollContainerRef.scrollTop
          scrollContainerRef.scrollTop = Math.max(
            0,
            currentScroll - SCROLL_SPEED * intensity
          )

          // Update selection even when outside viewport
          updateSelectionAtPosition(mouseY)
        }
      }, 16) // 60fps
    }
    // Check if near bottom edge or below container
    else if (mouseY > effectiveBottom - EDGE_THRESHOLD) {
      const distanceFromBottom = Math.max(0, effectiveBottom - mouseY)
      const intensity =
        1 - Math.max(0, Math.min(1, distanceFromBottom / EDGE_THRESHOLD))

      autoScrollInterval = window.setInterval(() => {
        if (scrollContainerRef) {
          const currentScroll = scrollContainerRef.scrollTop
          const maxScroll =
            scrollContainerRef.scrollHeight - scrollContainerRef.clientHeight
          scrollContainerRef.scrollTop = Math.min(
            maxScroll,
            currentScroll + SCROLL_SPEED * intensity
          )

          // Update selection even when outside viewport
          updateSelectionAtPosition(mouseY)
        }
      }, 16) // 60fps
    }
  }

  // Update selection based on mouse Y position
  const updateSelectionAtPosition = (mouseY: number) => {
    if (!tableRef || !isDragging()) return

    const rows = tableRef.querySelectorAll("[data-row-id]")
    for (const row of rows) {
      const rect = row.getBoundingClientRect()
      if (mouseY >= rect.top && mouseY <= rect.bottom) {
        const rowId = row.getAttribute("data-row-id")
        if (rowId) {
          updatePreviewSelection(rowId)
        }
        break
      }
    }
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

    // Stop auto-scrolling
    if (autoScrollInterval) {
      clearInterval(autoScrollInterval)
      autoScrollInterval = null
    }
  }

  // Context menu event handlers
  const handleContextMenu = (rowId: string, _event: MouseEvent) => {
    const selectedRowIds = Object.keys(props.rowSelection()).filter(
      (id) => props.rowSelection()[id]
    )

    if (!selectedRowIds.includes(rowId)) {
      // Clear all selections and select only the right-clicked item
      const newSelection = { [rowId]: true }
      props.setRowSelection(newSelection)
      setContextMenuSelection(new Set([rowId]))
    } else {
      // Row is already selected, keep current selection
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

  const getContextMenuItems = (): {
    type: "item" | "separator" | "submenu"
    label?: string
    action?: () => void
    icon?: string
    id?: string
    destructive?: boolean
    disabled?: boolean
    children?: {
      type: "item" | "separator"
      label?: string
      action?: () => void
      icon?: string
      id?: string
      destructive?: boolean
      disabled?: boolean
    }[]
  }[] => {
    const selectedCount = contextMenuSelection().size
    const selectedIds = Array.from(contextMenuSelection())
    const mods = rows()
      .filter((row) => selectedIds.includes(row.id))
      .map((row) => row.original)

    if (selectedCount === 1 && mods.length > 0) {
      // Single item menu
      const mod = mods[0]
      const displayName = mod.metadata?.name || mod.filename

      const items: {
        type: "item" | "separator" | "submenu"
        label?: string
        action?: () => void
        icon?: string
        id?: string
        destructive?: boolean
        disabled?: boolean
        children?: {
          type: "item" | "separator"
          label?: string
          action?: () => void
          icon?: string
          id?: string
          destructive?: boolean
          disabled?: boolean
        }[]
      }[] = [
        {
          type: "item",
          label: t("instance.copy_name"),
          action: () => {
            navigator.clipboard.writeText(displayName)
            toast.success(t("instance.copied_to_clipboard"))
          },
          icon: "i-hugeicons:clipboard",
          id: "copy"
        },
        { type: "separator" },
        {
          type: "item",
          label: mod.enabled
            ? t("instance.disable_mod")
            : t("instance.enable_mod"),
          action: async () => {
            if (props.mutations) {
              await props.mutations.handleToggleMod(mod)
            }
          },
          icon: mod.enabled
            ? "i-hugeicons:toggle-off"
            : "i-hugeicons:toggle-off",
          id: "toggle",
          disabled: props.isInstanceLocked()
        }
      ]

      if (mod.has_update) {
        items.push({
          type: "item",
          label: t("instance.update_mod"),
          action: async () => {
            if (props.mutations) {
              await props.mutations.handleUpdateMod(mod)
            }
          },
          icon: "i-hugeicons:download-02",
          id: "update",
          disabled: props.isInstanceLocked()
        })
      }

      items.push(
        { type: "separator" },
        {
          type: "item",
          label: t("instance.view_details"),
          action: () => {
            modalsContext?.openModal(
              {
                name: "modDetails"
              },
              {
                mod,
                instanceId: props.mutations?.instanceId
              }
            )
          },
          icon: "i-hugeicons:information-circle",
          id: "details"
        },
        {
          type: "item",
          label: t("instance.open_folder"),
          action: () => {
            if (props.mutations) {
              props.mutations.handleOpenFolder()
            }
          },
          icon: "i-hugeicons:folder-open",
          id: "folder"
        }
      )

      if (mod.curseforge || mod.modrinth) {
        // If both platforms exist, show them as a submenu
        if (mod.curseforge && mod.modrinth) {
          items.push({
            type: "submenu",
            label: t("instance.view_on_platform"),
            icon: "i-hugeicons:link-square-02",
            id: "platform",
            children: [
              {
                type: "item",
                label: t("instance.view_on_curseforge"),
                action: () => {
                  const slug = mod.curseforge!.urlslug
                  window.open(
                    `https://www.curseforge.com/minecraft/mc-mods/${slug}`,
                    "_blank"
                  )
                },
                icon: "curseforge",
                id: "platform-curseforge"
              },
              {
                type: "item",
                label: t("instance.view_on_modrinth"),
                action: () => {
                  window.open(
                    `https://modrinth.com/mod/${mod.modrinth!.project_id}`,
                    "_blank"
                  )
                },
                icon: "modrinth",
                id: "platform-modrinth"
              }
            ]
          })
        } else {
          // Single platform
          const platformName = mod.curseforge ? "curseforge" : "modrinth"
          items.push({
            type: "item",
            label: t(`instance.view_on_${platformName}`),
            action: () => {
              if (mod.curseforge) {
                const slug = mod.curseforge.urlslug
                window.open(
                  `https://www.curseforge.com/minecraft/mc-mods/${slug}`,
                  "_blank"
                )
              } else if (mod.modrinth) {
                window.open(
                  `https://modrinth.com/mod/${mod.modrinth.project_id}`,
                  "_blank"
                )
              }
            },
            icon: platformName,
            id: "platform"
          })
        }
      }

      items.push({ type: "separator" })

      // Add delete item at the end with extra separator for spacing
      const deleteItem = {
        type: "item" as const,
        label: t("instance.delete_mod"),
        action: async () => {
          if (props.mutations) {
            await props.mutations.handleDeleteMod(mod)
          }
        },
        destructive: true,
        icon: "i-hugeicons:delete-02",
        id: "delete",
        disabled: props.isInstanceLocked()
      }

      // Add some spacing items if platform submenu exists
      if (mod.curseforge || mod.modrinth) {
        items.push({ type: "separator" })
      }

      items.push(deleteItem)

      return items
    } else {
      // Multiple items menu
      const allEnabled = mods.every((mod) => mod.enabled)
      const allDisabled = mods.every((mod) => !mod.enabled)

      const items: {
        type: "item" | "separator" | "submenu"
        label?: string
        action?: () => void
        icon?: string
        id?: string
        destructive?: boolean
        disabled?: boolean
        children?: {
          type: "item" | "separator"
          label?: string
          action?: () => void
          icon?: string
          id?: string
          destructive?: boolean
          disabled?: boolean
        }[]
      }[] = [
        {
          type: "item",
          label: t("instance.selected_count", { count: selectedCount }),
          disabled: true,
          id: "header"
        },
        { type: "separator" }
      ]

      if (!allEnabled) {
        items.push({
          type: "item",
          label: t("instance.enable_all"),
          action: async () => {
            if (props.mutations) {
              await Promise.all(
                mods
                  .filter((mod) => !mod.enabled)
                  .map((mod) => props.mutations!.handleToggleMod(mod))
              )
            }
          },
          icon: "i-hugeicons:toggle-off",
          id: "enable-all",
          disabled: props.isInstanceLocked()
        })
      }

      if (!allDisabled) {
        items.push({
          type: "item",
          label: t("instance.disable_all"),
          action: async () => {
            if (props.mutations) {
              await Promise.all(
                mods
                  .filter((mod) => mod.enabled)
                  .map((mod) => props.mutations!.handleToggleMod(mod))
              )
            }
          },
          icon: "i-hugeicons:toggle-off",
          id: "disable-all",
          disabled: props.isInstanceLocked()
        })
      }

      // Add update option if any selected mods have updates
      const hasUpdates = mods.some((mod) => mod.has_update)
      if (hasUpdates) {
        items.push({
          type: "item",
          label: t("instance.update_selected"),
          action: async () => {
            if (props.mutations) {
              await props.mutations.handleUpdateSelected(mods)
            }
          },
          icon: "i-hugeicons:download-02",
          id: "update-selected",
          disabled: props.isInstanceLocked()
        })
      }

      items.push(
        { type: "separator" },
        {
          type: "item",
          label: t("instance.delete_selected"),
          action: async () => {
            if (props.mutations) {
              await props.mutations.handleDeleteSelected(mods)
            }
          },
          destructive: true,
          icon: "i-hugeicons:delete-02",
          id: "delete",
          disabled: props.isInstanceLocked()
        }
      )

      return items
    }
  }

  const getRowClasses = (rowId: string) => {
    const baseClasses =
      "border-darkSlate-600 flex w-full border-t group/row cursor-pointer"

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

  const createScrollHandler = (_container: HTMLElement) => {
    let animationFrameId: number | null = null
    let lastKnownScrollTop = 0

    const updateScrollPosition = () => {
      if (!tableRef || !headerRef) return

      const headerRect = headerRef.getBoundingClientRect()
      const tableRect = tableRef.getBoundingClientRect()

      // Calculate scroll offset from the bottom of the header
      const headerBottom = headerRect.bottom
      const tableTop = tableRect.top
      const scrollOffset = headerBottom - tableTop
      const newScrollTop = Math.max(0, scrollOffset)

      // Always update if there's a significant change
      if (Math.abs(newScrollTop - lastKnownScrollTop) > 0.1) {
        lastKnownScrollTop = newScrollTop
        setScrollTop(newScrollTop)
      }
    }

    const handleScroll = () => {
      // Cancel any pending update
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId)
      }

      // Schedule immediate update
      updateScrollPosition()

      // Also schedule one more update after animation frame
      // This catches any missed updates during fast scrolling
      animationFrameId = requestAnimationFrame(() => {
        updateScrollPosition()
        animationFrameId = null
      })
    }

    return {
      handler: handleScroll,
      cleanup: () => {
        if (animationFrameId !== null) {
          cancelAnimationFrame(animationFrameId)
        }
      }
    }
  }

  onMount(() => {
    if (props.onTableReady) {
      props.onTableReady(table)
    }

    // Initialize scroll handling
    scrollHandlerCleanup?.()
    resizeObserver?.disconnect()

    const container = findScrollContainer()
    if (container) {
      scrollContainerRef = container

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

      // Use both scroll and scrollend for better updates
      container.addEventListener("scroll", handler, { passive: true })

      // scrollend fires when scrolling momentum stops - ensures final update
      if ("onscrollend" in container) {
        container.addEventListener("scrollend", handler, { passive: true })
      }

      scrollHandlerCleanup = () => {
        container.removeEventListener("scroll", handler)
        if ("onscrollend" in container) {
          container.removeEventListener("scrollend", handler)
        }
        cleanup()
      }
    }

    document.addEventListener("mouseup", handleMouseUp)
    document.addEventListener("mousemove", handleMouseMove)
  })

  onCleanup(() => {
    scrollHandlerCleanup?.()
    resizeObserver?.disconnect()
    document.removeEventListener("mouseup", handleMouseUp)
    document.removeEventListener("mousemove", handleMouseMove)
    document.body.style.userSelect = ""
    document.body.style.cursor = ""
    if (autoScrollInterval) {
      clearInterval(autoScrollInterval)
    }
  })

  return (
    <ContextMenu onOpenChange={handleContextMenuOpenChange}>
      <ContextMenuTrigger class="border-darkSlate-600 rounded-lg border">
        <div
          ref={headerRef}
          class="bg-darkSlate-700 sticky z-10 rounded-t-lg"
          style={{ top: `${props.headerTopOffset ?? 170}px` }}
        >
          <For each={table.getHeaderGroups()}>
            {(headerGroup) => (
              <div class="flex">
                <For each={headerGroup.headers}>
                  {(header) => (
                    <div
                      class="text-lightSlate-300 min-w-0 px-4 py-3 text-left text-sm font-medium"
                      classList={{
                        "flex-1": !header.getSize() || header.getSize() === 150,
                        "flex-shrink-0":
                          !!header.getSize() && header.getSize() !== 150
                      }}
                      style={
                        header.getSize() && header.getSize() !== 150
                          ? { width: `${header.getSize()}px` }
                          : undefined
                      }
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
                                class="text-xs"
                                classList={{
                                  "i-hugeicons:arrow-up-01": true,
                                  "text-blue-400":
                                    header.column.getIsSorted() === "asc",
                                  "text-lightSlate-500":
                                    header.column.getIsSorted() !== "asc"
                                }}
                              />
                              <div
                                class="-mt-1 text-xs"
                                classList={{
                                  "i-hugeicons:arrow-down-01": true,
                                  "text-blue-400":
                                    header.column.getIsSorted() === "desc",
                                  "text-lightSlate-500":
                                    header.column.getIsSorted() !== "desc"
                                }}
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

        <div ref={tableRef} onMouseLeave={handleMouseLeave}>
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
                        class="flex min-w-0 items-center px-4 py-3 text-sm"
                        classList={{
                          "flex-1":
                            !cell.column.getSize() ||
                            cell.column.getSize() === 150,
                          "flex-shrink-0":
                            !!cell.column.getSize() &&
                            cell.column.getSize() !== 150
                        }}
                        style={
                          cell.column.getSize() && cell.column.getSize() !== 150
                            ? { width: `${cell.column.getSize()}px` }
                            : undefined
                        }
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
                when={item && item.type === "separator"}
                fallback={
                  <Show
                    when={item && item.type === "submenu"}
                    fallback={
                      item && (
                        <ContextMenuItem
                          disabled={item.disabled}
                          class={
                            item.destructive
                              ? "text-red-400 focus:text-red-300"
                              : ""
                          }
                          onSelect={item.action}
                        >
                          <div class="flex items-center gap-2">
                            <Show when={item.icon}>
                              <Show
                                when={item.icon === "curseforge"}
                                fallback={
                                  <Show
                                    when={item.icon === "modrinth"}
                                    fallback={
                                      <div class={`${item.icon} h-4 w-4`} />
                                    }
                                  >
                                    <img
                                      src={ModrinthLogo}
                                      class="h-4 w-4"
                                      alt="Modrinth"
                                    />
                                  </Show>
                                }
                              >
                                <img
                                  src={CurseforgeLogo}
                                  class="h-4 w-4"
                                  alt="CurseForge"
                                />
                              </Show>
                            </Show>
                            <span>{item.label}</span>
                          </div>
                        </ContextMenuItem>
                      )
                    }
                  >
                    <ContextMenuSub gutter={8} shift={-5}>
                      <ContextMenuSubTrigger class="data-[state=open]:bg-darkSlate-700 relative">
                        <div class="flex items-center gap-2">
                          <Show when={item.icon}>
                            <div class={`${item.icon} h-4 w-4`} />
                          </Show>
                          <span>{item.label}</span>
                        </div>
                      </ContextMenuSubTrigger>
                      <ContextMenuPortal>
                        <ContextMenuSubContent class="z-[210]">
                          <For each={item.children || []}>
                            {(child) => (
                              <Show
                                when={child.type === "separator"}
                                fallback={
                                  <ContextMenuItem
                                    disabled={child.disabled}
                                    class={
                                      child.destructive
                                        ? "text-red-400 focus:text-red-300"
                                        : ""
                                    }
                                    onSelect={child.action}
                                  >
                                    <div class="flex items-center gap-2">
                                      <Show when={child.icon}>
                                        <Show
                                          when={child.icon === "curseforge"}
                                          fallback={
                                            <Show
                                              when={child.icon === "modrinth"}
                                              fallback={
                                                <div
                                                  class={`${child.icon} h-4 w-4`}
                                                />
                                              }
                                            >
                                              <img
                                                src={ModrinthLogo}
                                                class="h-4 w-4"
                                                alt="Modrinth"
                                              />
                                            </Show>
                                          }
                                        >
                                          <img
                                            src={CurseforgeLogo}
                                            class="h-4 w-4"
                                            alt="CurseForge"
                                          />
                                        </Show>
                                      </Show>
                                      <span>{child.label}</span>
                                    </div>
                                  </ContextMenuItem>
                                }
                              >
                                <ContextMenuSeparator />
                              </Show>
                            )}
                          </For>
                        </ContextMenuSubContent>
                      </ContextMenuPortal>
                    </ContextMenuSub>
                  </Show>
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
