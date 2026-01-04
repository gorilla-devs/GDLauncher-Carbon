import { Show, createMemo, createEffect, onCleanup } from "solid-js"
import { Portal } from "solid-js/web"
import { Popover as PopoverPrimitive } from "@kobalte/core/popover"
import { useOnboarding } from "./OnboardingContext"
import { Trans } from "@gd/i18n"

export const SpotlightOverlay = () => {
  const onboarding = useOnboarding()
  const activeTip = () => onboarding.activeTip()

  // Calculate clip-path polygon for the cutout
  // Creates a polygon that covers the entire screen except the target area
  const clipPath = createMemo(() => {
    const tip = activeTip()
    if (!tip) return "none"

    const rect = tip.targetRect
    const padding = 8 // Padding around the target element

    // Coordinates with padding
    const left = rect.left - padding
    const top = rect.top - padding
    const right = rect.right + padding
    const bottom = rect.bottom + padding

    // Create a polygon using evenodd fill rule
    // Outer rectangle (entire viewport) + inner rectangle (cutout)
    // The evenodd rule creates a "hole" where the rectangles overlap
    return `polygon(
      evenodd,
      0 0, 100vw 0, 100vw 100vh, 0 100vh, 0 0,
      ${left}px ${top}px, ${left}px ${bottom}px, ${right}px ${bottom}px, ${right}px ${top}px, ${left}px ${top}px
    )`
  })

  // Handle escape key
  createEffect(() => {
    if (!activeTip()) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onboarding.hideTip()
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown))
  })

  // Anchor position styles based on target rect
  const anchorStyle = createMemo(() => {
    const tip = activeTip()
    if (!tip) return {}
    return {
      left: `${tip.targetRect.left}px`,
      top: `${tip.targetRect.top}px`,
      width: `${tip.targetRect.width}px`,
      height: `${tip.targetRect.height}px`
    }
  })

  // Map placement to Kobalte placement
  const placement = createMemo(() => {
    const tip = activeTip()
    return tip?.config.placement ?? "bottom"
  })

  return (
    <Show when={activeTip()}>
      <Portal>
        {/* Overlay with cutout */}
        <div
          class="fixed inset-0 z-99999 bg-black/70 transition-opacity duration-200"
          style={{
            "clip-path": clipPath()
          }}
          onClick={() => onboarding.hideTip()}
        />

        {/* Popover using Kobalte with virtual anchor */}
        <PopoverPrimitive
          open={!!activeTip()}
          gutter={4}
          placement={placement()}
        >
          {/* Virtual anchor positioned at target element */}
          <PopoverPrimitive.Anchor
            class="pointer-events-none fixed"
            style={anchorStyle()}
          />
          <PopoverPrimitive.Portal>
            <PopoverPrimitive.Content class="z-100000 w-80 rounded-lg border-2 border-solid border-primary-500 bg-darkSlate-800 p-4 text-lightSlate-200 shadow-xl outline-none data-[expanded]:animate-popoverEnter data-[closed]:animate-popoverLeave">
              <Show when={activeTip()?.config.title}>
                <PopoverPrimitive.Title class="mb-2 text-base font-semibold text-lightSlate-50">
                  {activeTip()?.config.title}
                </PopoverPrimitive.Title>
              </Show>
              <PopoverPrimitive.Description class="text-sm text-lightSlate-300">
                {activeTip()?.config.description}
              </PopoverPrimitive.Description>

              {/* Got it button */}
              <div class="mt-4 flex justify-end">
                <button
                  class="rounded bg-primary-500 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-primary-400"
                  onClick={() => onboarding.hideTip()}
                >
                  <Trans key="onboarding:_trn_got_it" />
                </button>
              </div>

              {/* Close button */}
              <PopoverPrimitive.CloseButton class="absolute right-3 top-3 rounded-sm opacity-70 transition-opacity hover:opacity-100 focus:outline-none">
                <div class="i-ri:close-line h-4 w-4" />
              </PopoverPrimitive.CloseButton>
            </PopoverPrimitive.Content>
          </PopoverPrimitive.Portal>
        </PopoverPrimitive>
      </Portal>
    </Show>
  )
}
