import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"

/**
 * Authentication method selection step
 *
 * Presents browser OAuth as the primary method with device code as a fallback option.
 * Uses a "Primary button + link" layout to emphasize the recommended approach.
 */

interface AuthMethodStepProps {
  /** Callback when browser authentication is selected */
  onBrowserAuth: () => void
  /** Callback when device code authentication is selected */
  onDeviceCodeAuth: () => void
  /** Whether authentication is currently loading */
  loading?: boolean
}

export function AuthMethodStep(props: AuthMethodStepProps) {
  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 text-center">
      {/* Microsoft icon */}
      <div class="bg-darkSlate-700 flex h-20 w-20 items-center justify-center rounded-2xl">
        <div class="i-hugeicons:microsoft h-12 w-12 text-lightSlate-50" />
      </div>

      {/* Heading and description */}
      <div class="flex flex-col gap-2">
        <h2 class="text-lightSlate-50 m-0 text-xl font-bold">
          <Trans key="auth:_trn_login.titles.sign_in_with_microsoft" />
        </h2>
        <p class="text-lightSlate-700 m-0 max-w-80 text-sm">
          <Trans key="auth:_trn_login.sign_in_with_microsoft_text" />
        </p>
      </div>

      {/* Primary action: Browser OAuth */}
      <div class="flex w-full max-w-80 flex-col gap-3">
        <Button
          size="large"
          variant="primary"
          fullWidth
          onClick={props.onBrowserAuth}
          loading={props.loading}
          disabled={props.loading}
        >
          <div class="i-hugeicons:microsoft h-4 w-4" />
          <Trans key="auth:_trn_login.sign_in" />
        </Button>

        {/* Recommended badge */}
        <div class="text-primary-400 flex items-center justify-center gap-1 text-xs font-medium">
          <div class="i-hugeicons:star h-3 w-3" />
          <span>
            <Trans key="auth:_trn_login.recommended" />
          </span>
        </div>
      </div>

      {/* Alternative method: Device Code */}
      <div class="border-darkSlate-600 flex flex-col gap-2 border-t pt-6">
        <p class="text-lightSlate-600 m-0 text-xs">
          <Trans key="auth:_trn_login.trouble_browser_signin" />
        </p>
        <button
          type="button"
          class="text-primary-400 hover:text-primary-300 cursor-pointer text-sm font-medium underline disabled:cursor-not-allowed disabled:opacity-50"
          onClick={(e) => {
            e.preventDefault()
            e.stopPropagation()
            props.onDeviceCodeAuth()
          }}
          disabled={props.loading}
        >
          <Trans key="auth:_trn_login.use_device_code_instead" />
        </button>
      </div>

      {/* Info about device code */}
      <div class="text-lightSlate-700 max-w-80 text-xs leading-relaxed">
        <p class="m-0">
          <Trans key="auth:_trn_login.device_code_explanation" />
        </p>
      </div>
    </div>
  )
}
