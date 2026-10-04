import { toast } from "@gd/ui"
import { errorToastCopyAction } from "./errorToastCopyAction"
import {
  extractErrorCodeCause,
  extractErrorDisplay,
  presentedGlobally
} from "./rspcErrorText"

// A mutation's own failure toast. Kept apart from rspcClient.ts so the many
// tests that mock the rspc client still render the dialogs that use it.

/** `meta` for a mutation whose `onError` calls `toastMutationError`. The
 *  global handler then leaves the plain cause to that toast instead of
 *  showing the same sentence a second time. */
export const LOCAL_ERROR_TOAST = { localErrorToast: true }

/** A mutation's failure toast: its translated `title`, with the core's plain
 *  sentence beneath (never the raw rspc payload, which carries debug text
 *  and a backtrace) and the global error toast's Copy action for it. Shows
 *  nothing when the global handler presents the error itself, for example a
 *  server's recovered-process refusal, which it words in the player's
 *  language. */
export function toastMutationError(title: string, error: unknown) {
  if (presentedGlobally(extractErrorCodeCause(error))) return
  const description = extractErrorDisplay(error)
  toast.error(title, {
    description,
    action: errorToastCopyAction(description)
  })
}
