// Event bridge for INSUFFICIENT_MEMORY error handling
// Since rspcClient runs outside SolidJS tree, we use custom events

export const INSUFFICIENT_MEMORY_EVENT = "gdl:insufficient-memory"

export interface InsufficientMemoryData {
  instance_id: number
  requested_mb: number
  available_mb: number
}

// Dispatch from rspcClient when INSUFFICIENT_MEMORY detected
export function dispatchMemoryWarningEvent(data: InsufficientMemoryData) {
  window.dispatchEvent(
    new CustomEvent(INSUFFICIENT_MEMORY_EVENT, { detail: data })
  )
}

// Listen for the event in SolidJS tree
export function listenMemoryWarning(
  callback: (data: InsufficientMemoryData) => void
) {
  const handler = (e: Event) => {
    callback((e as CustomEvent<InsufficientMemoryData>).detail)
  }
  window.addEventListener(INSUFFICIENT_MEMORY_EVENT, handler)
  return () => window.removeEventListener(INSUFFICIENT_MEMORY_EVENT, handler)
}
