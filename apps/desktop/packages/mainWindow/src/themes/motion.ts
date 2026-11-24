import { createSignal } from "solid-js"
import type { ListInstance } from "@gd/core_module/bindings"

export type MotionState = "run" | "pause" | "off"

export function motionState(i: {
  reducedMotion: boolean
  osReducedMotion: boolean
  visible: boolean
  gameRunning: boolean
}): MotionState {
  if (i.reducedMotion || i.osReducedMotion) return "off"
  if (!i.visible || i.gameRunning) return "pause"
  return "run"
}

// Written inline: utils/instances.tsx has JSX, which the desktop vitest can't load.
export function isGameRunning(instances: ListInstance[] | undefined): boolean {
  return (
    instances?.some(
      (i) =>
        i.status.status === "valid" && i.status.value.state.state === "running"
    ) ?? false
  )
}

// Before initMotion runs, start from what the page can already know (the OS
// preference and the window's visibility) instead of a blind "run".
function initialState(): MotionState {
  const hasWindow = typeof window !== "undefined"
  return motionState({
    reducedMotion: false,
    osReducedMotion:
      hasWindow && typeof window.matchMedia === "function"
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false,
    visible:
      typeof document === "undefined" || document.visibilityState === "visible",
    gameRunning: false
  })
}

const [motionSignal, setMotionSignal] =
  createSignal<MotionState>(initialState())

export function motion(): MotionState {
  return motionSignal()
}

export function setMotion(state: MotionState): void {
  setMotionSignal(state)
}
