import { Trans } from "@gd/i18n"
import { createEffect, createSignal } from "solid-js"

interface Props {
  value?: string
  icon?: string
  onClick?: () => void
  handleRefresh?: () => void
  expired?: boolean
  id?: string
}

export const DeviceCode = (props: Props) => {
  let animateDivRef: HTMLDivElement | undefined
  const [copied, setCopied] = createSignal(false)
  const [refreshing, setRefreshing] = createSignal(false)

  function animateCopied() {
    setCopied(true)
    if (animateDivRef) {
      animateDivRef.animate([{ opacity: 0 }, { opacity: 100 }], {
        duration: 150,
        easing: "ease-spring",
        fill: "forwards"
      })

      setTimeout(() => {
        animateDivRef?.animate([{ opacity: 100 }, { opacity: 0 }], {
          duration: 150,
          easing: "ease-spring",
          fill: "forwards"
        })

        setTimeout(() => {
          setCopied(false)
        }, 150)
      }, 1000)
    }
  }

  createEffect(() => {
    if (refreshing() && !props.expired) {
      setRefreshing(false)
    }
  })

  return (
    <div class="h-13 text-lightSlate-50 bg-darkSlate-900 w-54 font-ubuntu border-1 border-lightSlate-900 relative flex items-center justify-center gap-2 overflow-hidden rounded-md border-solid font-bold opacity-100">
      <div
        ref={animateDivRef}
        class="z-1 absolute left-0 top-0 flex h-full w-full items-center justify-center bg-green-500 text-sm opacity-0"
        classList={{
          "translate-x-full": !copied(),
          "translate-x-0": copied()
        }}
      >
        <i class="i-hugeicons:tick-02 mr-2 h-4 w-4" />
        <Trans key="general:_trn_general_copied_to_clipboard" />
      </div>

      <span
        class="text-2xl font-normal"
        classList={{
          "text-lightSlate-50": !props.expired,
          "text-lightSlate-700": props.expired
        }}
      >
        {props.value}
      </span>
      <span
        id={props.id}
        class="transition-color hover:bg-lightSlate-50 duration-100 ease-spring"
        classList={{
          "i-hugeicons:copy-01": !props.expired,
          "i-hugeicons:refresh": props.expired,
          "animate-spin": refreshing(),
          "text-lightSlate-50": props.expired,
          "text-lightSlate-700": !props.expired
        }}
        onClick={async () => {
          if (props.expired) {
            if (props.handleRefresh) {
              setRefreshing(true)
              try {
                props.handleRefresh()
              } catch (e) {
                console.error(e)
              }
            }
          } else {
            window.copyToClipboard(props.value || "")
            animateCopied()

            if (props?.onClick && !props.expired) {
              props.onClick()
            }
          }
        }}
      />
    </div>
  )
}
