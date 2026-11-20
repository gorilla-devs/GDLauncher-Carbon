import { Button } from "@gd/ui"
import {
  JSX,
  createEffect,
  createSignal,
  on,
  onCleanup,
  onMount
} from "solid-js"
import { useModalStackEntry } from "."
import { CancelLink, DialogFooter } from "./DialogFooter"
import { focusWasDropped } from "./dialogFocus"

/** What a confirm dialog gets from whoever opens it. */
export interface ConfirmCallbacks {
  /** Does what the dialog asks about. The dialog stays open while it runs,
   *  and after a failure, whose error toast says why. */
  onConfirm: () => Promise<unknown>
  /** Runs once the dialog has closed, saying whether `onConfirm` ran to the
   *  end, so the opener can put focus back. */
  onClosed?: (confirmed: boolean) => void
}

/** A confirm dialog's state: busy while `onConfirm` runs, closed once it's
 *  done. It closes its own dialog, which another may have opened on top of
 *  while `onConfirm` ran. */
export function createConfirm(data: () => ConfirmCallbacks) {
  const stackEntry = useModalStackEntry()
  const [busy, setBusy] = createSignal(false)
  let confirmed = false

  onCleanup(() => data().onClosed?.(confirmed))

  const confirm = async () => {
    if (busy()) return
    setBusy(true)
    try {
      await data().onConfirm()
      confirmed = true
      stackEntry?.close()
    } catch (error) {
      // The mutation's error toast tells the user; this keeps the detail.
      console.error("Confirm action failed:", error)
      setBusy(false)
    }
  }

  return { busy, confirm, cancel: () => stackEntry?.close() }
}

/** "Cancel" as a quiet link, then the one primary button. These dialogs
 *  delete things, so Cancel takes focus: Enter or Esc leave everything as it
 *  was, and confirming takes a deliberate Tab or click. */
const ConfirmActions = (props: {
  busy: boolean
  /** Can't confirm right now; the dialog says why. */
  disabled?: boolean
  onConfirm: () => void
  onCancel: () => void
  /** Put on the primary button, for the dialogs that tests drive. */
  confirmTestId?: string
  children: JSX.Element
}) => {
  let cancel: HTMLButtonElement | undefined
  let primary: HTMLButtonElement | undefined

  onMount(() => cancel?.focus({ preventScroll: true }))
  // The button is disabled while busy, which drops focus; a failure gives it
  // back, unless focus already went somewhere else on purpose (a dialog's
  // error overlay with its own Dismiss button).
  createEffect(
    on(
      () => props.busy,
      (busy, wasBusy) => {
        if (
          wasBusy &&
          !busy &&
          focusWasDropped(document.activeElement, document.body, [
            primary,
            cancel
          ])
        )
          primary?.focus()
      },
      { defer: true }
    )
  )

  return (
    <DialogFooter>
      <CancelLink
        ref={(el) => (cancel = el)}
        disabled={props.busy}
        onClick={props.onCancel}
      />
      <Button
        ref={primary}
        loading={props.busy}
        disabled={props.disabled}
        data-testid={props.confirmTestId}
        onClick={() => props.onConfirm()}
      >
        {props.children}
      </Button>
    </DialogFooter>
  )
}

export default ConfirmActions
