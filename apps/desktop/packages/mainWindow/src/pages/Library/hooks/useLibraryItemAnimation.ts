/**
 * useLibraryItemAnimation Hook
 *
 * Extracts entrance and spring animation logic from LibraryItemTile.
 * Handles staggered entrance animations and newly created folder spring animations.
 */

import { Accessor, onMount, onCleanup } from "solid-js"
import { ANIMATION } from "../constants"

export interface UseLibraryItemAnimationOptions {
  /** Unique key for this item (used for animation tracking) */
  itemKey: string
  /** The ID of the item */
  itemId: number
  /** The type of item */
  itemType: "instance" | "folder" | "server"
  /** Ref accessor for the DOM element */
  ref: Accessor<HTMLDivElement | undefined>
  /** Index of this item in the list */
  itemIndex: Accessor<number>
  /** Whether reduced motion is enabled */
  reducedMotion: Accessor<boolean>
  /** Set of already-animated item IDs */
  animatedIds: Set<string>
  /** Whether initial animation is complete */
  initialComplete: { value: boolean }
  /** Total number of items in the list */
  itemsLength: Accessor<number>
  /** ID of a newly created folder (for spring animation) */
  newlyCreatedFolderId?: Accessor<number | null>
  /** Callback to clear the newly created folder ID */
  clearNewlyCreatedFolderId?: () => void
  /** Map to register library item refs for FLIP animation */
  libraryItemRefs: Map<string, HTMLDivElement>
  /** Map to register tile refs */
  tileRefs: Map<string, HTMLDivElement>
  /** Top-level item ID for tile refs */
  tileRefId: string
}

/**
 * Hook for managing library item entrance and spring animations.
 */
export function useLibraryItemAnimation(
  options: UseLibraryItemAnimationOptions
): void {
  const {
    itemKey,
    itemId,
    itemType,
    ref,
    itemIndex,
    reducedMotion,
    animatedIds,
    initialComplete,
    itemsLength,
    newlyCreatedFolderId,
    clearNewlyCreatedFolderId,
    libraryItemRefs,
    tileRefs,
    tileRefId
  } = options

  const isFolder = itemType === "folder"

  onMount(() => {
    const el = ref()
    if (!el) return

    // Register ref for FLIP animation
    libraryItemRefs.set(itemKey, el)
    tileRefs.set(tileRefId, el)

    const shouldAnimate =
      !reducedMotion() && !animatedIds.has(itemKey) && !initialComplete.value

    const isNewlyCreatedFolder = isFolder && newlyCreatedFolderId?.() === itemId

    if (shouldAnimate) {
      animatedIds.add(itemKey)
      const delay =
        ANIMATION.STAGGER_BASE + itemIndex() * ANIMATION.STAGGER_PER_ITEM
      const anim = el.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: ANIMATION.ENTRANCE_DURATION,
        delay,
        easing: "linear",
        fill: "both"
      })
      anim.onfinish = () => {
        el.style.opacity = "1"
      }
    } else if (isNewlyCreatedFolder && !reducedMotion()) {
      // Spring animation for newly created folders
      const anim = el.animate(
        [
          { transform: "scale(0.5)", opacity: 0 },
          { transform: "scale(1.05)", opacity: 1, offset: 0.7 },
          { transform: "scale(0.98)", opacity: 1, offset: 0.85 },
          { transform: "scale(1)", opacity: 1 }
        ],
        {
          duration: ANIMATION.SPRING_DURATION,
          easing: ANIMATION.SPRING_EASING,
          fill: "forwards"
        }
      )
      anim.onfinish = () => {
        el.style.opacity = "1"
        clearNewlyCreatedFolderId?.()
      }
    } else {
      // No animation — make visible immediately
      el.style.opacity = "1"
      if (isNewlyCreatedFolder) {
        clearNewlyCreatedFolderId?.()
      }
    }

    // Mark initial animation complete on last item
    if (itemIndex() === itemsLength() - 1) {
      requestAnimationFrame(() => {
        initialComplete.value = true
      })
    }
  })

  // Cleanup refs on unmount
  onCleanup(() => {
    libraryItemRefs.delete(itemKey)
    tileRefs.delete(tileRefId)
  })
}
