/** Press effect classes as a string (for use with cn() or class attribute) */
export const PRESS_CLASSES =
  "transition-all duration-100 ease-spring active:scale-95"
export const PRESS_CLASSES_LIGHT =
  "transition-all duration-100 ease-spring active:scale-98"
export const PRESS_CLASSES_DISABLED = "transition-all duration-100 ease-spring"

/**
 * Returns press effect classes as an object for use with classList
 * @param disabled - Whether the element is disabled
 */
export const getPressEffectClasses = (disabled: boolean) =>
  ({
    "transition-all": true,
    "duration-100": true,
    "ease-spring": true,
    "active:scale-95": !disabled
  }) as const
