/**
 * Library Hooks Index
 *
 * Re-exports all hooks for the Library view system.
 */

export { useLibraryData } from "./useLibraryData"
export type { UseLibraryDataReturn } from "./useLibraryData"

export { useLibrarySelection } from "./useLibrarySelection"

export { useFLIPAnimation, useEntranceAnimation } from "./useFLIPAnimation"

export { useLibraryDragDrop } from "./useLibraryDragDrop"

export { useDropIndicators } from "./useDropIndicators"
export type { UseDropIndicatorsOptions, DropIndicatorState } from "./useDropIndicators"

export { useDropZoneRegistration } from "./useDropZoneRegistration"
export type { UseDropZoneRegistrationOptions } from "./useDropZoneRegistration"

export { useLibraryItemAnimation } from "./useLibraryItemAnimation"
export type { UseLibraryItemAnimationOptions } from "./useLibraryItemAnimation"
