import { Show, children, createEffect, onCleanup, onMount } from "solid-js"
import { JSX } from "solid-js/jsx-runtime"
import { useTransContext } from "@gd/i18n"
import { ModalProps, useModal, useModalStackEntry } from "."
import adSize from "@/utils/adhelper"
import { DecorationSlot } from "@/themes/DecorationSlot"
import { identityHook } from "@/themes/hooks"

interface Props extends ModalProps {
  children: JSX.Element | Element
  class?: string
  preventClose?: boolean
  // Runs in place of closing the modal when its header's close button is pressed.
  onClose?: () => void
  noPadding?: boolean
  /** Starts the content right under the title, for a small dialog whose
   *  first line follows straight from it: the gap under the title then
   *  matches the one above the buttons. */
  compact?: boolean
  overflowHiddenDisabled?: boolean
  background?: JSX.Element
  height?: string
  width?: string
  scrollable?: string
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children)
  const [t] = useTransContext()
  const modalsContext = useModal()
  const stackEntry = useModalStackEntry()

  // Publishes this instance's live `preventClose` prop onto its stack entry
  // so the manager's Escape handler and backdrop click can see it too — the
  // manager also consults the static registry, but a prop-only
  // `preventClose` (e.g. a Java confirm dialog's while busy) has no registry
  // entry, so this is the only path that value reaches Escape/backdrop
  // through.
  onMount(() => {
    stackEntry?.registerPreventClose(() => props.preventClose === true)
  })

  // The dialog is labelled by the title while the header shows it.
  createEffect(() => stackEntry?.setTitleShown(!props.noHeader))

  onCleanup(() => {
    stackEntry?.unregisterPreventClose()
    stackEntry?.setTitleShown(false)
  })

  return (
    <div
      class={`${props.height || ""} ${props.width || ""}`}
      classList={{
        "h-full": !props.height,
        "w-full": !props.width
      }}
      style={{
        "max-width": props.width
          ? `calc(100vw - ${adSize.width + 40}px)`
          : undefined
      }}
      onClick={(e) => {
        e.stopPropagation()
      }}
    >
      <div
        data-gdl={
          stackEntry
            ? `modal ${identityHook("modal", stackEntry.name)}`
            : "modal"
        }
        class={`bg-darkSlate-700 relative isolate flex h-full origin-center flex-col rounded-2xl duration-100 ease-spring ${
          props.class ?? ""
        }`}
        classList={{
          "overflow-hidden": !props.overflowHiddenDisabled
        }}
      >
        <Show when={props.background}>{props.background}</Show>
        <Show when={!props.noHeader}>
          <div
            data-gdl="modal.header"
            class="box-border flex items-center justify-between px-5 pt-3 pb-4"
          >
            <h2
              id={stackEntry?.titleId}
              data-gdl="modal.title"
              class="text-lg font-bold text-lightSlate-50"
            >
              {props.title}
            </h2>
            <button
              type="button"
              data-testid="modal-close"
              data-gdl="modal.close"
              aria-label={t("general:_trn_close")}
              class="text-darkSlate-300 hover:text-lightSlate-100 b-0 flex h-5 w-5 shrink-0 cursor-pointer items-center justify-center rounded bg-transparent p-0 press-effect active:scale-90 outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
              onClick={() => {
                if (!props.preventClose) {
                  if (props.onClose) props.onClose()
                  else modalsContext?.closeModal()
                }
              }}
            >
              <div class="i-hugeicons:cancel-01 h-5 w-5" />
            </button>
          </div>
        </Show>
        <div
          data-gdl="modal.body"
          class={`box-border ${
            props.scrollable ? props.scrollable : "overflow-hidden"
          } z-10 h-full`}
          classList={{
            "pt-5": !props.compact,
            "px-5 pb-6": !props.noPadding
          }}
        >
          {c()}
        </div>
        <DecorationSlot anchor="modal" />
      </div>
    </div>
  )
}

export default ModalLayout
