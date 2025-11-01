import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { createSignal, Show, For } from "solid-js"
import { rspc } from "@/utils/rspcClient"

/**
 * Modal for optional GDL Account setup
 *
 * This is a simplified modal component that can be opened from the CompleteStep.
 * The full GDL account flow (GDLAccount, GDLAccountCompletion, GDLAccountVerification)
 * will be integrated here during the main index.tsx refactor.
 *
 * For now, this provides a basic structure and can be shown/hidden.
 */

interface GDLAccountSetupModalProps {
  /** Whether the modal is open */
  isOpen: boolean
  /** Callback to close the modal */
  onClose: () => void
  /** Callback when setup is completed */
  onComplete?: () => void
  /** Active account UUID */
  activeUuid?: string | null
}

enum ModalStep {
  Info = 1,
  Completion = 2,
  Verification = 3
}

export function GDLAccountSetupModal(props: GDLAccountSetupModalProps) {
  const [currentStep, setCurrentStep] = createSignal(ModalStep.Info)

  // Mutation to save GDL account skip state
  const saveGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.saveGdlAccount"]
  }))

  /**
   * Close modal and reset state
   */
  const handleClose = () => {
    setCurrentStep(ModalStep.Info)
    props.onClose()
  }

  /**
   * Skip GDL account setup
   * Saves empty string to indicate user explicitly skipped
   */
  const handleSkip = async () => {
    try {
      // Save "" to indicate user skipped GDL account setup
      await saveGdlAccountMutation.mutateAsync("")
      handleClose()
      // Trigger onComplete to proceed to library
      props.onComplete?.()
    } catch (error) {
      console.error("Failed to save skip state:", error)
      // Still close and continue even if save fails
      handleClose()
      props.onComplete?.()
    }
  }

  /**
   * Proceed to next step
   */
  const handleNext = () => {
    if (currentStep() < ModalStep.Verification) {
      setCurrentStep(currentStep() + 1)
    }
  }

  /**
   * Go back to previous step
   */
  const handleBack = () => {
    if (currentStep() > ModalStep.Info) {
      setCurrentStep(currentStep() - 1)
    }
  }

  if (!props.isOpen) return null

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div class="bg-darkSlate-800 relative flex h-[600px] w-full max-w-2xl flex-col rounded-xl border border-darkSlate-600 shadow-2xl">
        {/* Header */}
        <div class="border-darkSlate-600 flex items-center justify-between border-b px-6 py-4">
          <h2 class="text-lightSlate-50 text-lg font-bold">
            <Trans key="login.gdl_account_setup_title" />
          </h2>
          <button
            onClick={handleClose}
            class="text-lightSlate-600 hover:text-lightSlate-50 transition-colors"
          >
            <div class="i-hugeicons:cancel-01 h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div class="flex-1 overflow-y-auto p-6">
          <Show when={currentStep() === ModalStep.Info}>
            {/* Step 1: Information and benefits */}
            <div class="flex flex-col gap-6">
              <div class="flex items-center gap-4">
                <div class="bg-primary-500/20 flex h-16 w-16 shrink-0 items-center justify-center rounded-2xl">
                  <div class="i-hugeicons:computer-phone-sync h-8 w-8 text-primary-400" />
                </div>
                <div class="flex flex-col gap-1">
                  <h3 class="text-lightSlate-50 m-0 text-lg font-semibold">
                    <Trans key="login.unlock_cloud_features" />
                  </h3>
                  <p class="text-lightSlate-600 m-0 text-sm">
                    <Trans key="login.create_account_description" />
                  </p>
                </div>
              </div>

              <div class="flex flex-col gap-3">
                <h4 class="text-lightSlate-50 m-0 text-sm font-semibold">
                  <Trans key="login.benefits_label" />
                </h4>
                <ul class="text-lightSlate-600 flex flex-col gap-2 text-sm">
                  <li class="flex items-start gap-2">
                    <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-1 shrink-0" />
                    <span>
                      <Trans key="login.benefit_share_instances_friends" />
                    </span>
                  </li>
                  <li class="flex items-start gap-2">
                    <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-1 shrink-0" />
                    <span>
                      <Trans key="login.benefit_track_metrics_playtime" />
                    </span>
                  </li>
                  <li class="flex items-start gap-2">
                    <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-1 shrink-0" />
                    <span>
                      <Trans key="login.benefit_sync_settings_preferences" />
                    </span>
                  </li>
                  <li class="flex items-start gap-2">
                    <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-1 shrink-0" />
                    <span>
                      <Trans key="login.benefit_access_anywhere" />
                    </span>
                  </li>
                </ul>
              </div>

              <div class="border-darkSlate-600 bg-darkSlate-700/50 rounded-lg border p-4">
                <p class="text-lightSlate-600 m-0 text-xs">
                  <Trans key="login.account_security_notice" />
                </p>
              </div>
            </div>
          </Show>

          <Show when={currentStep() === ModalStep.Completion}>
            {/* Step 2: Email and nickname */}
            <div class="flex flex-col gap-4">
              <p class="text-lightSlate-600 text-sm">
                <Trans key="login.enter_recovery_email_nickname" />
              </p>
              {/* TODO: Integrate GDLAccountCompletion component here */}
              <div class="text-lightSlate-500 text-center text-sm">
                [GDL Account Completion Form - To be integrated]
              </div>
            </div>
          </Show>

          <Show when={currentStep() === ModalStep.Verification}>
            {/* Step 3: Email verification */}
            <div class="flex flex-col gap-4">
              <p class="text-lightSlate-600 text-sm">
                <Trans key="login.check_email_verification" />
              </p>
              {/* TODO: Integrate GDLAccountVerification component here */}
              <div class="text-lightSlate-500 text-center text-sm">
                [GDL Account Verification - To be integrated]
              </div>
            </div>
          </Show>
        </div>

        {/* Footer */}
        <div class="border-darkSlate-600 flex items-center justify-between border-t px-6 py-4">
          <Button
            size="medium"
            type="secondary"
            onClick={currentStep() === ModalStep.Info ? handleSkip : handleBack}
          >
            {currentStep() === ModalStep.Info ? (
              <Trans key="general.skip" />
            ) : (
              <>
                <div class="i-hugeicons:arrow-left-01" />
                <Trans key="general.back" />
              </>
            )}
          </Button>

          <div class="flex gap-2">
            {/* Step indicator */}
            <div class="flex items-center gap-1">
              <For each={[1, 2, 3]}>
                {(step) => (
                  <div
                    class="h-2 w-2 rounded-full"
                    classList={{
                      "bg-primary-400": step === currentStep(),
                      "bg-darkSlate-600": step !== currentStep()
                    }}
                  />
                )}
              </For>
            </div>

            <Button
              size="medium"
              onClick={
                currentStep() === ModalStep.Verification
                  ? props.onComplete
                  : handleNext
              }
            >
              {currentStep() === ModalStep.Verification ? (
                <Trans key="general.finish" />
              ) : (
                <>
                  <Trans key="general.next" />
                  <div class="i-hugeicons:arrow-right-01" />
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
