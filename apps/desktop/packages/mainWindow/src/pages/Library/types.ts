/**
 * Library View System Types
 *
 * Two mutually exclusive view modes:
 * - Folders Mode (instancesGroupBy = null): Manual drag-drop, real folders, libraryPosition ordering
 * - Accordion Mode (instancesGroupBy != null): Read-only virtual grouping, optional sortBy within groups
 */

import { ListInstance, ListGroup } from "@gd/core_module/bindings"
import { Accessor } from "solid-js"
import { DragType } from "./DragContext"

/**
 * The current view mode of the library.
 * Determined by settings.instancesGroupBy being null or having a value.
 */
export type LibraryViewMode = "folders" | "accordion"

/**
 * A library item can be either an instance or a folder.
 * Used in folders mode for the flat list of items at the library root.
 */
export type LibraryItem =
  | { id: string; type: "instance"; data: ListInstance }
  | {
      id: string
      type: "folder"
      data: FolderData
    }

/**
 * Data for a folder in the library.
 */
export interface FolderData {
  id: number
  name: string
  libraryPosition: number | null
  instances: ListInstance[]
}

/**
 * A virtual group for accordion mode.
 * Groups are computed based on instancesGroupBy setting (modloader, gameVersion, modplatform).
 */
export interface VirtualGroup {
  id: string | number | null
  name: string
  instances: ListInstance[]
}

/**
 * Selection state interface for multi-select functionality.
 * Uses type-prefixed string IDs (e.g., "instance-5", "folder-3") to avoid
 * collisions between instances and folders that share numeric IDs.
 */
export interface SelectionState {
  selectedIds: Accessor<Set<string>>
  isSelected: (id: string) => boolean
  toggleSelection: (id: string) => void
  selectAll: (ids: string[]) => void
  clearSelection: () => void
}

/**
 * FLIP animation state for library item reordering.
 */
export interface FLIPAnimationState {
  /** Map of item IDs to their captured DOM rects */
  positionSnapshot: Map<string, DOMRect>
  /** Snapshot of item order before mutation (to detect changes) */
  orderSnapshot: string[] | null
  /** Whether an animation is currently pending/running */
  isAnimating: boolean
}

/**
 * FLIP animation hook return type.
 */
export interface FLIPAnimation {
  /** Register a DOM element ref for a library item */
  registerRef: (key: string, el: HTMLDivElement | undefined) => void
  /** Capture current positions before a reorder mutation */
  capturePositions: (orderKeys: string[]) => void
  /** Check if order changed and run FLIP animation if needed */
  animateIfOrderChanged: (newKeys: string[]) => void
  /** Whether animation is currently pending */
  isAnimating: Accessor<boolean>
  /** Clean up animation state */
  cleanup: () => void
}

/**
 * Entrance animation state for initial load.
 */
export interface EntranceAnimationState {
  /** Set of item IDs that have already been animated */
  animatedIds: Set<string>
  /** Whether initial entrance animation is complete */
  initialComplete: boolean
}

/**
 * Accordion grouping criteria (when instancesGroupBy is not null).
 */
export type AccordionGroupBy = "modloader" | "gameVersion" | "modplatform"

/**
 * Sort criteria for accordion mode (when instancesSortBy is not null).
 * In folders mode, instancesSortBy is null and libraryPosition is used.
 */
export type AccordionSortBy =
  | "name"
  | "lastPlayed"
  | "lastUpdated"
  | "created"
  | "gameVersion"
  | "mostPlayed"

/**
 * Helper to determine view mode from settings.
 * instancesGroupBy = null means folders mode.
 */
export function getViewMode(instancesGroupBy: string | null | undefined): LibraryViewMode {
  return instancesGroupBy === null || instancesGroupBy === undefined
    ? "folders"
    : "accordion"
}

/**
 * Props for the main HomeGrid orchestrator.
 */
export interface HomeGridProps {
  // Currently no external props - all state is internal
}

/**
 * Props for FoldersView component.
 */
export interface FoldersViewProps {
  libraryItems: LibraryItem[]
  defaultGroupId: number | null
  tileSize: Accessor<number>
  selection: SelectionState
  openFolderId: Accessor<number | null>
  onToggleFolder: (folderId: number) => Promise<void>
  onDragStart: (type: DragType, ids: number[], e: PointerEvent) => void
  justDropped: Accessor<boolean>
  flipAnimation: FLIPAnimation
  entranceAnimation: EntranceAnimationState
}

/**
 * Props for AccordionView component.
 */
export interface AccordionViewProps {
  virtualGroups: VirtualGroup[]
  defaultGroupId: number | null
  tileSize: Accessor<number>
  selection: SelectionState
  onDragStart: (type: DragType, ids: number[], e: PointerEvent) => void
  justDropped: Accessor<boolean>
}

/**
 * Props for LibraryHeader component.
 */
export interface LibraryHeaderProps {
  filter: Accessor<string>
  setFilter: (value: string) => void
  tileSize: Accessor<number>
  setTileSize: (size: number) => void
  viewMode: Accessor<LibraryViewMode>
}
