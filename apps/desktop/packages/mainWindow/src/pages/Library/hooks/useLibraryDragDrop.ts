/**
 * useLibraryDragDrop Hook
 *
 * Manages drag-drop mutations for library items.
 * Integrates with FLIP animations and selection state.
 */

import { Accessor, createSignal, batch } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { DropTarget, DragType } from "../DragContext"
import { SelectionState, FLIPAnimation, LibraryItem } from "../types"

interface UseLibraryDragDropOptions {
  /** Default group ID for ungrouped instances */
  defaultGroupId: Accessor<number | null>
  /** Selection state for multi-select drag */
  selection: SelectionState
  /** FLIP animation hook for capturing positions */
  flipAnimation: FLIPAnimation
  /** Current library items for order snapshot */
  libraryItems: LibraryItem[]
  /** Callback when drop completes */
  onAfterDrop?: () => void
}

/**
 * Hook for handling library drag-drop operations.
 */
export function useLibraryDragDrop(options: UseLibraryDragDropOptions) {
  const globalStore = useGlobalStore()

  // Track newly created folder for spring animation
  const [newlyCreatedFolderId, setNewlyCreatedFolderId] = createSignal<
    number | null
  >(null)

  // Create all mutations
  const moveInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.moveInstance"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setFavorite"]
  }))

  const setServerFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  const moveGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.moveGroup"]
  }))

  const createFolderFromInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.createFolderFromInstances"],
    onSuccess: (groupId: number) => setNewlyCreatedFolderId(groupId)
  }))

  const arrangeLibraryMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.arrangeLibrary"]
  }))

  const moveServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.moveServer"]
  }))

  const moveServerGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["server.moveServerGroup"]
  }))

  const createFolderFromServersMutation = rspc.createMutation(() => ({
    mutationKey: ["server.createFolderFromServers"],
    onSuccess: (groupId: number) => setNewlyCreatedFolderId(groupId)
  }))

  /**
   * Handle instance drop events.
   * Mutations are batched to trigger a single reconciliation pass.
   */
  const handleInstanceDrop = (
    target: DropTarget,
    draggedIds: number[]
  ): void => {
    const _defaultGroupId = options.defaultGroupId()

    batch(() => {
      switch (target.type) {
        case "beforeInstance": {
          // Move instances before target instance
          for (const id of draggedIds) {
            if (id !== target.instanceId) {
              moveInstanceMutation.mutate({
                instance: id,
                target: { BeforeInstance: target.instanceId }
              })
            }
          }
          break
        }

        case "endOfGroup":
        case "folderContentArea": {
          // Move instances to end of group
          for (const id of draggedIds) {
            moveInstanceMutation.mutate({
              instance: id,
              target: { EndOfGroup: target.groupId }
            })
          }
          break
        }

        case "dropOnFolder": {
          // Move instances into folder (group)
          for (const id of draggedIds) {
            moveInstanceMutation.mutate({
              instance: id,
              target: { EndOfGroup: target.groupId }
            })
          }
          break
        }

        case "createFolder": {
          // Create new folder with the target instance and all dragged instances
          const allInstanceIds = [
            target.instanceId,
            ...draggedIds.filter((id) => id !== target.instanceId)
          ]
          createFolderFromInstancesMutation.mutate({
            instances: allInstanceIds,
            targetInstanceId: target.instanceId
          })
          break
        }

        case "ungrouped": {
          // Move instances back to default group
          if (_defaultGroupId) {
            for (const id of draggedIds) {
              moveInstanceMutation.mutate({
                instance: id,
                target: { EndOfGroup: _defaultGroupId }
              })
            }
          }
          break
        }

        case "beforeInstanceAtFolder": {
          // Move instances to default group, positioned before the folder
          for (const id of draggedIds) {
            moveInstanceMutation.mutate({
              instance: id,
              target: { BeforeGroup: target.folderId }
            })
          }
          break
        }
      }

      // Clear selection after drop
      options.selection.clearSelection()
    })
  }

  /**
   * Handle server drop events (mirrors handleInstanceDrop with server mutations).
   */
  const handleServerDrop = (target: DropTarget, draggedIds: number[]): void => {
    const _defaultGroupId = options.defaultGroupId()

    batch(() => {
      switch (target.type) {
        case "beforeInstance": {
          for (const id of draggedIds) {
            if (id !== target.instanceId) {
              moveServerMutation.mutate({
                server: id,
                target: { beforeServer: target.instanceId }
              })
            }
          }
          break
        }

        case "endOfGroup":
        case "folderContentArea": {
          for (const id of draggedIds) {
            moveServerMutation.mutate({
              server: id,
              target: { endOfGroup: target.groupId }
            })
          }
          break
        }

        case "dropOnFolder": {
          for (const id of draggedIds) {
            moveServerMutation.mutate({
              server: id,
              target: { endOfGroup: target.groupId }
            })
          }
          break
        }

        case "createFolder": {
          const allServerIds = [
            target.instanceId,
            ...draggedIds.filter((id) => id !== target.instanceId)
          ]
          createFolderFromServersMutation.mutate({
            servers: allServerIds,
            targetServerId: target.instanceId
          })
          break
        }

        case "ungrouped": {
          if (_defaultGroupId) {
            for (const id of draggedIds) {
              moveServerMutation.mutate({
                server: id,
                target: { endOfGroup: _defaultGroupId }
              })
            }
          }
          break
        }

        case "beforeInstanceAtFolder": {
          for (const id of draggedIds) {
            moveServerMutation.mutate({
              server: id,
              target: { beforeGroup: target.folderId }
            })
          }
          break
        }
      }

      options.selection.clearSelection()
    })
  }

  /**
   * Handle group drop events.
   */
  const handleGroupDrop = (target: DropTarget, groupId: number): void => {
    switch (target.type) {
      case "beforeGroup": {
        // Move group before target group
        if (groupId !== target.groupId) {
          moveGroupMutation.mutate({
            group: groupId,
            target: { BeforeGroup: target.groupId }
          })
        }
        break
      }

      case "beforeGroupAtInstance": {
        // Move group before an ungrouped instance
        moveGroupMutation.mutate({
          group: groupId,
          target: { BeforeInstance: target.beforeInstanceId }
        })
        break
      }

      case "endOfGroups":
      case "endOfLibrary": {
        // Move group to end of library
        moveGroupMutation.mutate({
          group: groupId,
          target: "EndOfLibrary"
        })
        break
      }
    }
  }

  /**
   * Handle server-group drop events.
   */
  const handleServerGroupDrop = (
    target: DropTarget,
    groupId: number
  ): void => {
    switch (target.type) {
      case "beforeGroup": {
        if (groupId !== target.groupId) {
          moveServerGroupMutation.mutate({
            group: groupId,
            target: { beforeGroup: target.groupId }
          })
        }
        break
      }

      case "beforeGroupAtInstance": {
        moveServerGroupMutation.mutate({
          group: groupId,
          target: { beforeServer: target.beforeInstanceId }
        })
        break
      }

      case "endOfGroups":
      case "endOfLibrary": {
        moveServerGroupMutation.mutate({
          group: groupId,
          target: "endOfLibrary"
        })
        break
      }
    }
  }

  /**
   * Main drop handler.
   */
  const handleDrop = (
    target: DropTarget | null,
    draggedIds: number[],
    dragType: DragType,
    origin: string | null
  ): void => {
    if (draggedIds.length === 0) return

    const isServerDrag = dragType === "server"

    // Favorites-origin: dropping outside bar = unfavorite
    if (origin === "favorites") {
      if (target?.type === "favorites") {
        // Dropped back on bar — no-op
        return
      }

      // Capture positions before mutation
      options.flipAnimation.capturePositions(
        options.libraryItems.map((item) => item.id)
      )

      // Unfavorite all dragged items
      for (const id of draggedIds) {
        if (isServerDrag) {
          setServerFavoriteMutation.mutate({ id, favorite: false })
        } else {
          setFavoriteMutation.mutate({ instance: id, favorite: false })
        }
      }
      options.selection.clearSelection()
      options.onAfterDrop?.()
      return
    }

    // Normal flow (grid-origin) — require a target
    if (!target) return

    // For "favorites" target from grid: only ADD favorites, never remove
    if (target.type === "favorites") {
      if (isServerDrag) {
        const draggedServers = (globalStore.servers.data || []).filter((s) =>
          draggedIds.includes(s.id)
        )
        for (const srv of draggedServers) {
          if (!srv.favorite) {
            setServerFavoriteMutation.mutate({ id: srv.id, favorite: true })
          }
        }
      } else {
        const draggedInstances = (globalStore.instances.data || []).filter(
          (i) => draggedIds.includes(i.id)
        )
        for (const inst of draggedInstances) {
          if (!inst.favorite) {
            setFavoriteMutation.mutate({ instance: inst.id, favorite: true })
          }
        }
      }
      options.selection.clearSelection()
      options.onAfterDrop?.()
      return
    }

    // Capture positions and order BEFORE mutation for FLIP animation
    options.flipAnimation.capturePositions(
      options.libraryItems.map((item) => item.id)
    )

    if (dragType === "instance") {
      handleInstanceDrop(target, draggedIds)
    } else if (dragType === "server") {
      handleServerDrop(target, draggedIds)
    } else if (dragType === "group") {
      handleGroupDrop(target, draggedIds[0])
    } else if (dragType === "serverGroup") {
      handleServerGroupDrop(target, draggedIds[0])
    }

    options.onAfterDrop?.()
  }

  return {
    handleDrop,
    newlyCreatedFolderId,
    clearNewlyCreatedFolderId: () => setNewlyCreatedFolderId(null),
    mutations: {
      moveInstance: moveInstanceMutation,
      setFavorite: setFavoriteMutation,
      moveGroup: moveGroupMutation,
      createFolder: createFolderFromInstancesMutation,
      arrangeLibrary: arrangeLibraryMutation,
      moveServer: moveServerMutation,
      moveServerGroup: moveServerGroupMutation,
      createServerFolder: createFolderFromServersMutation
    }
  }
}
