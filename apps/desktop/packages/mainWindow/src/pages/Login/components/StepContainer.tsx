import { type JSX } from "solid-js"
import type { TransitionDirection } from "../flow/types"

export interface StepContainerProps {
  children: JSX.Element
  direction?: TransitionDirection
  ref?: (el: HTMLDivElement) => void
}

/**
 * StepContainer
 *
 * Wrapper for step content that handles view transitions
 * Applies CSS for horizontal slide animations
 */
export function StepContainer(props: StepContainerProps) {
  return (
    <div
      ref={props.ref}
      class="flex h-full w-full justify-center overflow-y-auto overflow-x-hidden"
      style={{
        "view-transition-name": "step-content",
        // @ts-expect-error - view-transition-class is not in TypeScript types yet
        "view-transition-class": props.direction || "forward"
      }}
    >
      {props.children}
    </div>
  )
}
