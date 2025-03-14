const scrollPositions = new Map<string, number>()

const generateScrollKey = (element: Element, additionalKey?: string) => {
  const pathname = window.location.hash.slice(1) || "/"
  const elementId = element.id ? `#${element.id}` : ""
  const extraKey = additionalKey ? `:${additionalKey}` : ""
  return `${pathname}${elementId}${extraKey}`
}

export const saveScrollPosition = (
  element: Element | null,
  additionalKey?: string
) => {
  if (!element) return
  const key = generateScrollKey(element, additionalKey)
  console.log("Saving scroll position for", key, element.scrollTop)
  scrollPositions.set(key, element.scrollTop)
}

export const restoreScrollPosition = (
  element: Element | null,
  additionalKey?: string
) => {
  if (!element) return
  const key = generateScrollKey(element, additionalKey)
  const savedPosition = scrollPositions.get(key)
  console.log("Restoring scroll position for", key, savedPosition)
  if (savedPosition !== undefined) {
    element.scrollTop = savedPosition
  }
}

export const clearScrollPosition = (
  element: Element | null,
  additionalKey?: string
) => {
  if (!element) return
  const key = generateScrollKey(element, additionalKey)
  scrollPositions.delete(key)
}

export const clearAllScrollPositions = () => {
  scrollPositions.clear()
}
