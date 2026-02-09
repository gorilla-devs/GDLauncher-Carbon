interface DropOverlayIndicatorProps {
  isVisible: boolean
  icon: string
  class?: string
}

export function DropOverlayIndicator(props: DropOverlayIndicatorProps) {
  return (
    <div
      class={`border-primary-500 pointer-events-none absolute inset-0 flex items-center justify-center rounded-2xl border-2 transition-all duration-150 ease-out motion-reduce:transition-none ${props.class ?? ""}`}
      classList={{
        "opacity-100 bg-primary-500/20": props.isVisible,
        "opacity-0 bg-transparent": !props.isVisible
      }}
    >
      <div
        class="bg-primary-500 flex h-12 w-12 items-center justify-center rounded-full transition-transform duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)] motion-reduce:transition-none"
        classList={{
          "scale-100": props.isVisible,
          "scale-0": !props.isVisible
        }}
      >
        <div class={`${props.icon} text-3xl text-white`} />
      </div>
    </div>
  )
}
