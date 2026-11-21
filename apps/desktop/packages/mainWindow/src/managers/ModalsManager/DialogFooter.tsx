import { Trans } from "@gd/i18n"
import { JSX } from "solid-js"
import { TextLink } from "@/components/TextLink"

/** The row at the bottom of every dialog: quiet links first, then secondary
 *  buttons, then the one primary button on the right. A row too long wraps
 *  upwards, the buttons on top. */
export const DialogFooter = (props: {
  children: JSX.Element
  class?: string
}) => (
  <div
    data-gdl="modal.footer"
    class={`flex flex-wrap-reverse items-center justify-end gap-x-6 gap-y-3${
      props.class ? ` ${props.class}` : ""
    }`}
  >
    {props.children}
  </div>
)

/** A dialog's "Cancel", the quiet link for backing out (rule 1), beside its
 *  one primary button. */
export const CancelLink = (props: {
  disabled?: boolean
  onClick: () => void
  ref?: (el: HTMLButtonElement) => void
}) => (
  <TextLink ref={props.ref} disabled={props.disabled} onClick={props.onClick}>
    <Trans key="general:_trn_cancel" />
  </TextLink>
)
