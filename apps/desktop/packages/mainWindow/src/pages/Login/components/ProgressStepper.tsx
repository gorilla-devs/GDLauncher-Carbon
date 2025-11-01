import { For } from "solid-js"

/**
 * Progress stepper component for Login flow
 *
 * Displays a visual progress indicator with dots and animated progress bar.
 */

interface ProgressStepperProps {
  /** Current step number (1-indexed) */
  currentStep: number
  /** Total number of steps */
  totalSteps: number
}

export function ProgressStepper(props: ProgressStepperProps) {
  /**
   * Calculate progress bar position
   * Transforms the bar from left to show current progress
   */
  const progressBarStyle = () => {
    const stepIndex = props.currentStep
    const totalSteps = props.totalSteps
    const progressPercent = (100 * stepIndex) / totalSteps
    const offsetPixels = (stepIndex === 1 ? 7 : 6) - stepIndex

    return {
      transform: `translateX(calc((-100% + ${progressPercent}%) - ${offsetPixels}px))`,
      transition: "transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)"
    }
  }

  return (
    <div class="relative mb-4 flex justify-center gap-2">
      {/* Progress bar background */}
      <div class="absolute left-0 top-1/2 h-4 w-full -translate-y-1/2 overflow-hidden rounded-lg">
        <div
          class="bg-darkSlate-400 absolute left-0 top-0 h-4 w-full rounded-lg"
          style={progressBarStyle()}
        />
      </div>

      {/* Progress dots */}
      <For each={Array(props.totalSteps).fill(0)}>
        {() => (
          <div class="z-1 flex h-4 w-4 items-center justify-center">
            <div class="bg-lightSlate-900 h-2 w-2 rounded-full" />
          </div>
        )}
      </For>
    </div>
  )
}
