import type { AuthStep } from "../flow/types"

/**
 * ErrorStep
 *
 * Displays error message when initialization or data loading fails.
 * Retry button is now in the footer for consistency.
 */

interface ErrorStepProps {
  step: Extract<AuthStep, { type: "error" }>
}

export function ErrorStep(props: ErrorStepProps) {
  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-6 p-6 text-center">
      {/* Error icon */}
      <div class="bg-red-500/10 flex h-24 w-24 items-center justify-center rounded-full">
        <div class="i-hugeicons:alert-circle h-12 w-12 text-red-400" />
      </div>

      {/* Error message */}
      <p class="text-lightSlate-400 m-0 max-w-md text-base leading-relaxed">
        {props.step.message}
      </p>
    </div>
  )
}
