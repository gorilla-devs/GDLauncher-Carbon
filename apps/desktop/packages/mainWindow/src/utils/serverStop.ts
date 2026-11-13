/**
 * How long a server has to have been stopping before Force stop is offered.
 * Most servers are done well within it, so the control never flashes up.
 */
export const FORCE_STOP_AFTER_SECONDS = 10

/** Whether a stop that has run for `seconds` offers Force stop. */
export function offersForceStop(seconds: number | null | undefined) {
  return seconds != null && seconds >= FORCE_STOP_AFTER_SECONDS
}

/** `seconds` as a clock: 0:04, 1:07, 1:02:03. */
export function stopClock(seconds: number) {
  const whole = Math.max(0, Math.floor(seconds))
  const hours = Math.floor(whole / 3600)
  const minutes = Math.floor((whole % 3600) / 60)
  const rest = String(whole % 60).padStart(2, "0")
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${rest}`
    : `${minutes}:${rest}`
}
