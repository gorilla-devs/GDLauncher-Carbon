import {
  createEffect,
  onMount,
  onCleanup,
  JSX,
  children as resolveChildren,
  splitProps
} from "solid-js"
import { useOnboarding, registerTip, unregisterTip } from "./OnboardingContext"
import type { OnboardingTipConfig, OnboardingTrigger } from "./types"

interface OnboardingTipProps extends Omit<OnboardingTipConfig, "trigger"> {
  children: JSX.Element
  /** Trigger mode (default: onClick) */
  trigger?: OnboardingTrigger
}

export const OnboardingTip = (props: OnboardingTipProps) => {
  const [local, tipConfig] = splitProps(props, ["children"])
  const onboarding = useOnboarding()
  let wrapperRef: HTMLDivElement | undefined

  const config: OnboardingTipConfig = {
    id: tipConfig.id,
    title: tipConfig.title,
    description: tipConfig.description,
    trigger: tipConfig.trigger ?? "onClick",
    delay: tipConfig.delay,
    placement: tipConfig.placement,
    requiresFirstLaunchComplete: tipConfig.requiresFirstLaunchComplete ?? true
  }

  // Register tip on mount
  onMount(() => {
    registerTip(config.id, config)
  })

  onCleanup(() => {
    unregisterTip(config.id)
  })

  // Get bounding rect of first child element (wrapper has display:contents)
  const getTargetRect = (): DOMRect | null => {
    if (!wrapperRef) return null
    // With display:contents, get the first actual child element
    const firstChild = wrapperRef.firstElementChild as HTMLElement | null
    if (firstChild) {
      return firstChild.getBoundingClientRect()
    }
    return wrapperRef.getBoundingClientRect()
  }

  const triggerTip = () => {
    // TODO: Remove bypass after testing
    // if (!onboarding.isEnabled()) return
    // if (onboarding.isTipSeen(config.id)) return

    const rect = getTargetRect()
    if (rect) {
      onboarding.showTip(config.id, rect)
    }
  }

  // Handle onMount trigger
  createEffect(() => {
    if (config.trigger !== "onMount") return
    if (!onboarding.isEnabled()) return
    if (onboarding.isTipSeen(config.id)) return

    const delay = config.delay ?? 500
    const timer = setTimeout(() => {
      triggerTip()
    }, delay)

    onCleanup(() => clearTimeout(timer))
  })

  // Handle onClick trigger with capturing to intercept before children
  const handleClick = () => {
    if (config.trigger !== "onClick") return

    // Show tip on first interaction if not seen (don't block the click)
    if (!onboarding.isTipSeen(config.id)) {
      // Delay to allow any expansion animations to complete before capturing rect
      const delay = config.delay ?? 0
      if (delay > 0) {
        setTimeout(() => triggerTip(), delay)
      } else {
        triggerTip()
      }
    }
  }

  // Use capturing phase to intercept clicks before they reach children
  onMount(() => {
    if (config.trigger === "onClick" && wrapperRef) {
      const firstChild = wrapperRef.firstElementChild as HTMLElement | null
      if (firstChild) {
        firstChild.addEventListener("click", handleClick, { capture: true })
        onCleanup(() =>
          firstChild.removeEventListener("click", handleClick, {
            capture: true
          })
        )
      }
    }
  })

  const c = resolveChildren(() => local.children)

  return (
    <div ref={wrapperRef} style={{ display: "contents" }}>
      {c()}
    </div>
  )
}

/**
 * Hook for manual triggering of onboarding tips
 * @param tipId - The ID of the tip to trigger
 * @returns An object with setRef (to set the target element) and trigger (to show the tip)
 */
export const useTriggerOnboardingTip = (tipId: string) => {
  const onboarding = useOnboarding()
  let elementRef: HTMLElement | null = null

  const setRef = (el: HTMLElement) => {
    elementRef = el
  }

  const trigger = () => {
    if (!elementRef) return
    if (onboarding.isTipSeen(tipId)) return

    const rect = elementRef.getBoundingClientRect()
    onboarding.showTip(tipId, rect)
  }

  return { setRef, trigger }
}
