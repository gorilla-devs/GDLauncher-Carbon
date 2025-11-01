import { useLocation } from "@solidjs/router"
import { Show, children } from "solid-js"
import { JSX } from "solid-js/jsx-runtime"
import { ModalProps, useModal } from "."
import { useGDNavigate } from "../NavigationManager"
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
  const navigator = useGDNavigate()
  const location = useLocation()
  const modalsContext = useModal()

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
        class={`bg-darkSlate-700 relative flex h-full origin-center flex-col rounded-2xl duration-100 ease-in-out ${
          props.class ?? ""
        }`}
        classList={{
          "overflow-hidden": !props.overflowHiddenDisabled
        }}
      >
        <Show when={props.background}>{props.background}</Show>
        <Show when={!props.noHeader}>
          <div class="bg-darkSlate-800 box-border flex h-12 items-center justify-between rounded-t-2xl px-5">
            <h3>{props.title}</h3>
            <div
              class="text-darkSlate-300 i-hugeicons:cancel-01 hover:text-lightSlate-100 h-5 w-5 duration-100 ease-in-out"
              onClick={() => {
                if (!props.preventClose) {
                  navigator.navigate(location.pathname)
                  modalsContext?.closeModal()
                }
              }}
            />
          </div>
        </Show>
        <div
          class={`box-border ${
            props.scrollable ? props.scrollable : "overflow-hidden"
          } z-10 h-full`}
          classList={{
            "p-5": !props.noPadding
          }}
        >
          {c()}
        </div>
      </div>
    </div>
  )
}

export default ModalLayout
