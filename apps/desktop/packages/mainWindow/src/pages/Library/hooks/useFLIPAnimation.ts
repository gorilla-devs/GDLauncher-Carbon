/**
 * useFLIPAnimation Hook
 *
 * Component-scoped FLIP (First, Last, Invert, Play) animation hook.
 * Handles reorder animations without module-level mutable state.
 *
 * FLIP Animation Steps:
 * 1. FIRST: Capture positions before change
 * 2. LAST: Let DOM update (new positions)
 * 3. INVERT: Calculate delta and apply inverse transform
 * 4. PLAY: Animate from inverse to identity
 */

import { createSignal, onCleanup, Accessor } from "solid-js"
import { FLIPAnimation } from "../types"
import { ANIMATION } from "../constants"

interface UseFLIPAnimationOptions {
  /** Whether reduced motion is enabled */
  reducedMotion: Accessor<boolean>
}

/**
 * Hook for FLIP animations during library item reordering.
 * All state is component-scoped (no module-level mutable state).
 */
export function useFLIPAnimation(
  options: UseFLIPAnimationOptions
): FLIPAnimation {
  // Component-scoped refs map (cleaned up on unmount)
  const itemRefs = new Map<string, HTMLDivElement>()

  // Component-scoped state
  const [positionSnapshot, setPositionSnapshot] = createSignal<
    Map<string, DOMRect>
  >(new Map())
  const [orderSnapshot, setOrderSnapshot] = createSignal<string[] | null>(null)
  const [isAnimating, setIsAnimating] = createSignal(false)

  // Safety timeout ref
  let safetyTimeoutId: ReturnType<typeof setTimeout> | null = null

  /**
   * Register a DOM element ref for a library item.
   * Call this in the render function to track elements.
   */
  const registerRef = (key: string, el: HTMLDivElement | undefined): void => {
    if (el) {
      itemRefs.set(key, el)
    } else {
      itemRefs.delete(key)
    }
  }

  /**
   * Capture current positions of all registered items.
   * Call this BEFORE triggering a mutation that will reorder items.
   */
  const capturePositions = (orderKeys: string[]): void => {
    const snapshot = new Map<string, DOMRect>()

    itemRefs.forEach((el, key) => {
      if (el.isConnected) {
        snapshot.set(key, el.getBoundingClientRect())
      }
    })

    setPositionSnapshot(snapshot)
    setOrderSnapshot(orderKeys)
    setIsAnimating(true)

    // Safety timeout: cleanup if animation never runs
    if (safetyTimeoutId) {
      clearTimeout(safetyTimeoutId)
    }
    safetyTimeoutId = setTimeout(() => {
      cleanup()
    }, ANIMATION.SAFETY_TIMEOUT)
  }

  /**
   * Check if order changed and run FLIP animation if needed.
   * Call this AFTER the DOM has updated with new positions.
   */
  const animateIfOrderChanged = (newKeys: string[]): void => {
    const snapshot = orderSnapshot()
    if (!snapshot || !isAnimating()) return

    // Check if order actually changed
    const orderChanged =
      snapshot.length !== newKeys.length ||
      snapshot.some((id, i) => id !== newKeys[i])

    if (!orderChanged) {
      cleanup()
      return
    }

    // Clear safety timeout since we're running animation
    if (safetyTimeoutId) {
      clearTimeout(safetyTimeoutId)
      safetyTimeoutId = null
    }

    // Skip animation if reduced motion is enabled
    if (options.reducedMotion()) {
      cleanup()
      return
    }

    // Single RAF with staleness validation
    requestAnimationFrame(() => {
      // Verify elements still valid before animating
      const stillValid = Array.from(positionSnapshot().keys()).every((key) => {
        const el = itemRefs.get(key)
        return el && el.isConnected && el.offsetParent !== null
      })

      if (!stillValid) {
        // Elements were removed or hidden during the RAF delay
        cleanup()
        return
      }

      runFlipAnimation()
    })
  }

  /**
   * Run the actual FLIP animation.
   */
  const runFlipAnimation = (): void => {
    const oldPositions = positionSnapshot()
    const animations: Animation[] = []

    // Iterate over captured positions, not refs
    // This ensures items that were recreated during DOM reconciliation still animate
    oldPositions.forEach((oldRect, key) => {
      const el = itemRefs.get(key)
      if (!el?.isConnected) return

      const newRect = el.getBoundingClientRect()

      // Skip zero-size elements (layout thrash)
      if (newRect.width === 0 || newRect.height === 0) {
        el.style.opacity = "1"
        return
      }

      const dx = oldRect.left - newRect.left
      const dy = oldRect.top - newRect.top

      // Skip if no movement
      if (dx === 0 && dy === 0) {
        el.style.opacity = "1"
        return
      }

      // Apply FLIP animation
      const anim = el.animate(
        [
          { transform: `translate(${dx}px, ${dy}px)` },
          { transform: "translate(0, 0)" }
        ],
        {
          duration: ANIMATION.FLIP_DURATION,
          easing: ANIMATION.FLIP_EASING
        }
      )
      animations.push(anim)
    })

    // Ensure all elements visible after animation completes
    Promise.all(animations.map((a) => a.finished)).finally(() => {
      oldPositions.forEach((_, key) => {
        const el = itemRefs.get(key)
        if (el) el.style.opacity = "1"
      })
    })

    cleanup()
  }

  /**
   * Clean up animation state.
   */
  const cleanup = (): void => {
    if (safetyTimeoutId) {
      clearTimeout(safetyTimeoutId)
      safetyTimeoutId = null
    }
    setPositionSnapshot(new Map())
    setOrderSnapshot(null)
    setIsAnimating(false)
  }

  // Clean up on component unmount
  onCleanup(() => {
    if (safetyTimeoutId) {
      clearTimeout(safetyTimeoutId)
    }
    itemRefs.clear()
  })

  return {
    registerRef,
    capturePositions,
    animateIfOrderChanged,
    isAnimating,
    cleanup
  }
}

export interface EntranceAnimationReturn {
  /** Set of item IDs that have been animated (string for library items, number for instances) */
  animatedIds: Set<string | number>
  /** Whether initial animation is complete */
  initialComplete: boolean
  /** Mutable value property for compatibility with existing components */
  value: boolean
  /** Check if an item should animate on mount */
  shouldAnimate: (id: string | number) => boolean
  /** Mark initial animation as complete */
  markInitialComplete: () => void
  /** Check if an item has been animated */
  hasAnimated: (id: string | number) => boolean
  /** Reset animation state */
  reset: () => void
}

// Module-level state so entrance animation tracking persists across
// component mount/unmount cycles (e.g. navigating away and back).
const _animatedIds = new Set<string | number>()
const [_initialCompleteSignal, _setInitialCompleteSignal] = createSignal(false)

/**
 * Hook for entrance animations (staggered fade-in on initial load).
 * Uses module-scoped state so animations don't replay on navigation.
 */
export function useEntranceAnimation(): EntranceAnimationReturn {
  /**
   * Check if an item should animate on mount.
   */
  const shouldAnimate = (id: string | number): boolean => {
    if (_animatedIds.has(id) || _initialCompleteSignal()) {
      return false
    }
    _animatedIds.add(id)
    return true
  }

  /**
   * Mark initial animation as complete.
   */
  const markInitialComplete = (): void => {
    requestAnimationFrame(() => {
      _setInitialCompleteSignal(true)
    })
  }

  /**
   * Check if an item has been animated.
   */
  const hasAnimated = (id: string | number): boolean => {
    return _animatedIds.has(id) || _initialCompleteSignal()
  }

  /**
   * Reset animation state (e.g., when switching view modes).
   */
  const reset = (): void => {
    _animatedIds.clear()
    _setInitialCompleteSignal(false)
  }

  return {
    animatedIds: _animatedIds,
    get initialComplete() {
      return _initialCompleteSignal()
    },
    get value() {
      return _initialCompleteSignal()
    },
    set value(v: boolean) {
      _setInitialCompleteSignal(v)
    },
    shouldAnimate,
    markInitialComplete,
    hasAnimated,
    reset
  }
}
