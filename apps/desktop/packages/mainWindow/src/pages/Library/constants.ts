/**
 * Animation Constants
 *
 * Centralized animation timings and easing functions for the Library view.
 * These ensure consistent animations across all library components.
 */

export const ANIMATION = {
  /** FLIP animation duration in milliseconds */
  FLIP_DURATION: 300,
  /** FLIP animation easing function */
  FLIP_EASING: "ease-out",
  /** Base delay for staggered entrance animations */
  STAGGER_BASE: 100,
  /** Per-item delay for staggered entrance animations */
  STAGGER_PER_ITEM: 40,
  /** Per-group delay for staggered entrance animations */
  STAGGER_PER_GROUP: 60,
  /** Safety timeout to cleanup stuck animation state */
  SAFETY_TIMEOUT: 500,
  /** Entrance animation duration */
  ENTRANCE_DURATION: 250,
  /** Spring animation duration for newly created folders */
  SPRING_DURATION: 400,
  /** Spring animation easing */
  SPRING_EASING: "ease-out",
  /** Duration for drag layout FLIP animation */
  DRAG_LAYOUT_DURATION: 200,
  /** Spring-like easing for drag layout animation (fast out, gentle settle) */
  DRAG_LAYOUT_EASING: "cubic-bezier(0.25, 1, 0.5, 1)"
} as const

/**
 * Tile Size Configuration
 *
 * Maps tile size numbers (1-5) to CSS classes and dimensions.
 */
export const TILE_SIZES = {
  1: {
    container: "h-24 w-24",
    widthPx: 96,
    gapY: "gap-y-4",
    icon: "w-10 h-10"
  },
  2: {
    container: "h-46 w-46",
    widthPx: 184,
    gapY: "gap-y-6",
    icon: "w-20 h-20"
  },
  3: {
    container: "h-60 w-60",
    widthPx: 240,
    gapY: "gap-y-8",
    icon: "w-26 h-26"
  },
  4: {
    container: "h-84 w-84",
    widthPx: 336,
    gapY: "gap-y-10",
    icon: "w-38 h-38"
  },
  5: {
    container: "h-120 w-120",
    widthPx: 480,
    gapY: "gap-y-12",
    icon: "w-56 h-56"
  }
} as const

export type TileSize = keyof typeof TILE_SIZES
