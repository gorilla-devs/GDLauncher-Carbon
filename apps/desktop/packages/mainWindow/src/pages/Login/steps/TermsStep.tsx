import { Trans } from "@gd/i18n"
import { Checkbox } from "@gd/ui"
import { useModal } from "@/managers/ModalsManager"
import type { AuthStep } from "../flow/types"

/**
 * TermsStep
 *
 * Terms and Privacy acceptance step.
 * Shows CMP notice and requires checkbox acceptance before proceeding.
 * Supports both initial acceptance and forced re-acceptance for updated terms.
 */

interface TermsStepProps {
  step: Extract<AuthStep, { type: "terms" }>
  termsAccepted: boolean
  onTermsAcceptedChange: (accepted: boolean) => void
}

export function TermsStep(props: TermsStepProps) {
  const modalsContext = useModal()

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Content */}
      <div class="flex flex-col items-center justify-between gap-6 h-full">
        {/* Overwolf CMP Notice */}
        <div class="bg-darkSlate-700/20 border-darkSlate-600/50 max-w-md rounded-lg border px-6 py-4 text-left overflow-y-auto h-[60%] min-h-[150px] flex-shrink">
          <p class="text-lightSlate-400 m-0 mb-3 text-sm leading-relaxed">
            <Trans key="auth:_trn_login.cmp_notice_paragraph1" />
          </p>
          <p class="text-lightSlate-400 m-0 mb-3 text-sm leading-relaxed">
            <Trans key="auth:_trn_login.cmp_instructions" />
          </p>
          <p class="text-lightSlate-500 m-0 text-xs leading-relaxed">
            <Trans key="auth:_trn_login.purposes_we_use" />{" "}
            <Trans key="auth:_trn_login.we_value_privacy_text5" />
          </p>
        </div>

        {/* Acceptance Block */}
        <div class="bg-darkSlate-700/30 border-primary-500/50 flex w-full max-w-md flex-col items-center gap-4 rounded-lg border p-6">
          <Checkbox
            checked={props.termsAccepted}
            onChange={() => props.onTermsAcceptedChange(!props.termsAccepted)}
          >
            <span class="text-lightSlate-100 select-none text-base font-medium leading-relaxed ml-2">
              <Trans key="auth:_trn_login.terms_checkbox_label" />
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
              <Trans key="auth:_trn_login.terms_link_label" />
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
              <Trans key="auth:_trn_login.privacy_link_label" />
            </button>

            <span class="text-darkSlate-500">•</span>

            <button
              class="text-primary-400 hover:text-primary-300 flex items-center gap-1.5 whitespace-nowrap font-medium transition-colors"
              onClick={() => {
                window?.openCMPWindow()
              }}
            >
              <div class="i-hugeicons:settings-02 h-4 w-4 shrink-0" />
              <Trans key="auth:_trn_login.manage_cmp" />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
