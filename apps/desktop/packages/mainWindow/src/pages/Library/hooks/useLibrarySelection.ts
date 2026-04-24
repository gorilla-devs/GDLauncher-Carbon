/**
 * useLibrarySelection Hook
 *
 * Manages multi-select state for library items (instances and folders).
 * Uses type-prefixed string IDs (e.g., "instance-5", "folder-3") to avoid
 * collisions between instances and folders that share numeric IDs.
 */

import { createSignal } from "solid-js"
import { SelectionState } from "../types"

/**
 * Hook for managing multi-select state in the library.
 */
export function useLibrarySelection(): SelectionState {
  const [selectedIds, setSelectedIds] = createSignal<Set<string>>(new Set())

  const isSelected = (id: string): boolean => {
    return selectedIds().has(id)
  }

  const toggleSelection = (id: string): void => {
    setSelectedIds((prev) => {
      const newSet = new Set(prev)
      if (newSet.has(id)) {
        newSet.delete(id)
      } else {
        newSet.add(id)
      }
      return newSet
    })
  }

  const selectAll = (ids: string[]): void => {
    setSelectedIds(new Set(ids))
  }

  const clearSelection = (): void => {
    setSelectedIds(new Set<string>())
  }

  return {
    selectedIds,
    isSelected,
    toggleSelection,
    selectAll,
    clearSelection
  }
}
