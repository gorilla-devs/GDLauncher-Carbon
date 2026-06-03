/** Press effect classes as a string (for use with cn() or class attribute)
 * Uses 50ms scale transition to prevent text blur during scale animations (Modrinth approach)
 */
export const PRESS_CLASSES = "press-effect active:scale-95"
export const PRESS_CLASSES_LIGHT = "press-effect active:scale-98"
export const PRESS_CLASSES_DISABLED = "press-effect"

/**
 * Returns press effect classes as an object for use with classList
 * @param disabled - Whether the element is disabled
 */
export const getPressEffectClasses = (disabled: boolean) =>
  ({
    "press-effect": true,
    "active:scale-95": !disabled
  }) as const
