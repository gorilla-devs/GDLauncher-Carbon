import { createContext, useContext, type JSX } from "solid-js"
import type { AnimationController } from "../flow/types"
import { createAnimationController } from "./AnimationController"

/**
 * Animation Context
 * Provides access to the animation controller throughout the auth flow
 */
const AnimationContext = createContext<AnimationController>()

export interface AnimationProviderProps {
  children: JSX.Element
  reducedMotion: boolean
}

/**
 * AnimationProvider
 * Creates and provides the animation controller instance
 */
export function AnimationProvider(props: AnimationProviderProps) {
  const controller = createAnimationController(props.reducedMotion)

  return (
    <AnimationContext.Provider value={controller}>
      {props.children}
    </AnimationContext.Provider>
  )
}

/**
 * useAnimations hook
 * Access the animation controller from any component
 */
export function useAnimations(): AnimationController {
  const context = useContext(AnimationContext)
  if (!context) {
    throw new Error("useAnimations must be used within AnimationProvider")
  }
  return context
}
