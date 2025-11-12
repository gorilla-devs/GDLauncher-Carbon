import { Trans } from "@gd/i18n"
import { createSignal } from "solid-js"
import { Button } from "@gd/ui"
import { useFlow } from "../flow/FlowContext"

/**
 * AuthMethodStep
 *
 * Authentication method selection step.
 * Presents browser OAuth as the primary method with device code as a fallback.
 * Uses "Primary button + link" layout to emphasize the recommended approach.
 * All action buttons are in the content (not footer).
 */
export function AuthMethodStep() {
  const [loading, setLoading] = createSignal(false)
  const flow = useFlow()

  const handleBrowserAuth = async () => {
    setLoading(true)
    try {
      await flow.startEnrollment("browser")
      await flow.goToStep({
        type: "enrolling",
        method: "browser"
      })
    } catch (error) {
      console.error("Failed to start browser auth:", error)
      setLoading(false)
    }
  }

  const handleDeviceCodeAuth = async () => {
    setLoading(true)
    try {
      await flow.startEnrollment("device-code")
      await flow.goToStep({
        type: "enrolling",
        method: "device-code"
      })
    } catch (error) {
      console.error("Failed to start device code auth:", error)
      setLoading(false)
    }
  }

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Content */}
      <div class="flex flex-col items-center justify-center gap-6">
        {/* Microsoft icon */}
        <div class="bg-darkSlate-700 flex h-20 w-20 items-center justify-center rounded-2xl">
          <div class="i-hugeicons:microsoft h-12 w-12 text-lightSlate-50" />
        </div>

        {/* Description */}
        <p class="text-lightSlate-400 m-0 max-w-md text-base leading-relaxed">
          <Trans key="auth:_trn_login.sign_in_with_microsoft_text" />
        </p>

        {/* Sign-in buttons - moved from footer */}
        <div class="flex w-full max-w-md flex-col gap-3 mt-4">
          {/* Primary: Browser OAuth */}
          <Button
            size="large"
            variant="primary"
            fullWidth
            onClick={handleBrowserAuth}
            loading={loading()}
            disabled={loading()}
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

          {/* Alternative: Device Code */}
          <div class="border-darkSlate-600 flex flex-col gap-2 border-t pt-3">
            <p class="text-lightSlate-600 m-0 text-xs">
              <Trans key="auth:_trn_login.trouble_browser_signin" />
            </p>
            <button
              type="button"
              class="text-primary-400 hover:text-primary-300 cursor-pointer text-sm font-medium underline disabled:cursor-not-allowed disabled:opacity-50"
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                handleDeviceCodeAuth()
              }}
              disabled={loading()}
            >
              <Trans key="auth:_trn_login.use_device_code_instead" />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
