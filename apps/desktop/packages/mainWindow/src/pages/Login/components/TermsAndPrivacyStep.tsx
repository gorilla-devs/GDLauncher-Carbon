import { Trans } from "@gd/i18n"
import { Checkbox } from "@gd/ui"
import { useModal } from "@/managers/ModalsManager"
import { createSignal } from "solid-js"

/**
 * Terms and Privacy acceptance step
 *
 * Shows CMP notice and requires terms acceptance before proceeding
 */

interface TermsAndPrivacyStepProps {
  /** Initial checked state for the checkbox */
  initialAccepted?: boolean
  /** Callback when terms acceptance state changes */
  onAcceptanceChange?: (accepted: boolean) => void
}

export function TermsAndPrivacyStep(props: TermsAndPrivacyStepProps) {
  const modalsContext = useModal()
  const [termsAccepted, setTermsAccepted] = createSignal(
    props.initialAccepted ?? false
  )

  // Notify parent when acceptance state changes
  const handleAcceptanceChange = (accepted: boolean) => {
    setTermsAccepted(accepted)
    props.onAcceptanceChange?.(accepted)
  }

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-between gap-8 p-6 min-h-0">
      {/* Title */}
      <div class="flex flex-col items-center gap-2 text-center">
        <h2 class="text-lightSlate-50 m-0 text-2xl font-semibold">
          <Trans key="login.terms_title" />
        </h2>
        <p class="text-lightSlate-400 text-sm">
          <Trans key="login.terms_subtitle" />
        </p>
      </div>

      {/* Overwolf CMP Notice */}
      <div class="bg-darkSlate-700/20 border-darkSlate-600/50 max-w-md rounded-lg border px-6 py-4 text-left overflow-y-auto max-h-[400px] flex-shrink">
        <p class="text-lightSlate-400 m-0 mb-3 text-sm leading-relaxed">
          <Trans key="login.cmp_notice_paragraph1" />
        </p>
        <p class="text-lightSlate-400 m-0 mb-3 text-sm leading-relaxed">
          <Trans key="login.cmp_instructions" />
        </p>
        <p class="text-lightSlate-500 m-0 text-xs leading-relaxed">
          Purposes we use: <Trans key="login.we_value_privacy_text5" />
        </p>
      </div>

      {/* Acceptance Block */}
      <div class="bg-darkSlate-700/30 border-primary-500/50 flex w-full max-w-md flex-col items-center gap-4 rounded-lg border p-6">
        <Checkbox
          checked={termsAccepted()}
          onChange={() => handleAcceptanceChange(!termsAccepted())}
        >
          <span class="text-lightSlate-100 select-none text-base font-medium leading-relaxed ml-2">
            <Trans key="login.terms_checkbox_label" />
          </span>
        </Checkbox>

        {/* Inline Links */}
        <div class="text-lightSlate-500 flex items-center gap-3 text-sm">
          <button
            class="text-primary-400 hover:text-primary-300 flex items-center gap-1 transition-colors"
            onClick={() => {
              modalsContext?.openModal({
                name: "termsAndConditions"
              })
            }}
          >
            <div class="i-hugeicons:file-02 h-4 w-4 shrink-0" />
            <Trans key="login.terms_link_label" />
          </button>

          <span class="text-darkSlate-500">•</span>

          <button
            class="text-primary-400 hover:text-primary-300 flex items-center gap-1 transition-colors"
            onClick={() => {
              modalsContext?.openModal({
                name: "privacyStatement"
              })
            }}
          >
            <div class="i-hugeicons:shield-user h-4 w-4 shrink-0" />
            <Trans key="login.privacy_link_label" />
          </button>

          <span class="text-darkSlate-500">•</span>

          <button
            class="text-primary-400 hover:text-primary-300 flex items-center gap-1.5 whitespace-nowrap font-medium transition-colors"
            onClick={() => {
              window?.openCMPWindow()
            }}
          >
            <div class="i-hugeicons:settings-02 h-4 w-4 shrink-0" />
            <Trans key="login.manage_cmp" />
          </button>
        </div>
      </div>
    </div>
  )
}
