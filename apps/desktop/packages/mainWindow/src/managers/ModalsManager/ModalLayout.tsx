import { Show, children, onCleanup, onMount } from "solid-js"
import { JSX } from "solid-js/jsx-runtime"
import { ModalProps, useModal, useModalStackEntry } from "."
import adSize from "@/utils/adhelper"

interface Props extends ModalProps {
  children: JSX.Element | Element
  class?: string
  preventClose?: boolean
  noPadding?: boolean
  overflowHiddenDisabled?: boolean
  background?: JSX.Element
  height?: string
  width?: string
  scrollable?: string
}

const ModalLayout = (props: Props) => {
  const c = children(() => props.children)
  const modalsContext = useModal()
  const stackEntry = useModalStackEntry()

  // Publishes this instance's live `preventClose` prop onto its stack entry
  // so the manager's Escape handler and backdrop click can see it too — the
  // manager also consults the static registry, but a prop-only
  // `preventClose` (e.g. JavaSetup's) has no registry entry, so this is the
  // only path that value reaches Escape/backdrop through.
  onMount(() => {
    stackEntry?.registerPreventClose(() => props.preventClose === true)
  })

  onCleanup(() => {
    stackEntry?.unregisterPreventClose()
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
        class={`bg-darkSlate-700 relative flex h-full origin-center flex-col rounded-2xl duration-100 ease-spring ${
          props.class ?? ""
        }`}
        classList={{
          "overflow-hidden": !props.overflowHiddenDisabled
        }}
      >
        <Show when={props.background}>{props.background}</Show>
        <Show when={!props.noHeader}>
          <div class="box-border flex items-center justify-between px-5 pt-3 pb-4">
            <h2 class="text-lg font-bold text-lightSlate-50">{props.title}</h2>
            <div
              data-testid="modal-close"
              class="text-darkSlate-300 i-hugeicons:cancel-01 hover:text-lightSlate-100 h-5 w-5 press-effect active:scale-90 cursor-pointer"
              onClick={() => {
                if (!props.preventClose) {
                  modalsContext?.closeModal()
                }
              }}
            />
          </div>
        </Show>
        <div
          class={`box-border ${
            props.scrollable ? props.scrollable : "overflow-hidden"
          } z-10 h-full pt-5`}
          classList={{
            "px-5 pb-6": !props.noPadding
          }}
        >
          {c()}
        </div>
      </div>
    </div>
  )
}

export default ModalLayout
