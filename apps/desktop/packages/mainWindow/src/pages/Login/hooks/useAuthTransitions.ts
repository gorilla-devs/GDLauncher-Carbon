import { createSignal } from "solid-js"

/**
 * Hook for managing step transitions with View Transition API support
 *
 * Provides smooth horizontal transitions between authentication steps
 */
export function useAuthTransitions<TStepEnum extends number>(props: {
  initialStep: TStepEnum
  minStep: TStepEnum
  maxStep: TStepEnum
  shouldTransition?: () => boolean
}) {
  const [step, setStep] = createSignal<TStepEnum>(props.initialStep)
  const [transitionDirection, setTransitionDirection] = createSignal<
    "forward" | "backward"
  >("forward")

  // Check if View Transition API is available and should be used
  const shouldTransition = () => {
    if (props.shouldTransition) {
      return props.shouldTransition()
    }
    return typeof document !== "undefined" && "startViewTransition" in document
  }

  // Transition to a specific step with animation
  const transitionToStep = (newStep: TStepEnum) => {
    const currentStep = step()

    // Idempotent: Don't transition if already at target step
    if (currentStep === newStep) {
      return
    }

    const direction = newStep > currentStep ? "forward" : "backward"
    setTransitionDirection(direction)

    if (shouldTransition()) {
      // Use View Transition API for smooth animated transition
      // @ts-ignore - startViewTransition is not in TypeScript types yet
      document.startViewTransition(() => {
        setStep(() => newStep)
      })
    } else {
      // Fallback: instant transition
      setStep(() => newStep)
    }
  }

  // Go to next step
  const nextStep = () => {
    if (step() < props.maxStep) {
      transitionToStep((step() + 1) as TStepEnum)
    }
  }

  // Go to previous step
  const prevStep = () => {
    if (step() > props.minStep) {
      transitionToStep((step() - 1) as TStepEnum)
    }
  }

  // Check if we can go forward
  const canGoNext = () => step() < props.maxStep

  // Check if we can go backward
  const canGoBack = () => step() > props.minStep

  return {
    // State
    step,
    transitionDirection,

    // Methods
    transitionToStep,
    nextStep,
    prevStep,

    // Utilities
    canGoNext,
    canGoBack,
    setStep
  }
}
