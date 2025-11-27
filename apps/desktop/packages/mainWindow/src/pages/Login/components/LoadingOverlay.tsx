import { Spinner } from "@gd/ui"
import { Trans, NamespacedTranslationKey } from "@gd/i18n"
import { Show } from "solid-js"
import type { LoadingOperation } from "../flow/types"

export interface LoadingOverlayProps {
  operation: LoadingOperation
}

/**
 * LoadingOverlay
 *
 * Shows a centered spinner with operation-specific message
 * Displayed during global loading states (not in-step loading)
 */
export function LoadingOverlay(props: LoadingOverlayProps) {
  return (
    <div class="absolute inset-0 z-50 flex flex-col items-center justify-center gap-4">
      <Spinner class="h-10 w-10" />

      <Show when={getMessage(props.operation)}>
        {(message) => (
          <p class="text-lightSlate-500 text-sm">
            <Trans key={message()} />
          </p>
        )}
      </Show>
    </div>
  )
}

/**
 * Get translation key for loading message based on operation
 */
function getMessage(
  operation: LoadingOperation
): NamespacedTranslationKey | null {
  switch (operation) {
    case "initializing":
      return null // No message for initial load

    case "checking-account":
      return "auth:_trn_login.checking_account"

    case "checking-gdl":
      return "auth:_trn_login.checking_gdl_account"

    default:
      return null
  }
}
