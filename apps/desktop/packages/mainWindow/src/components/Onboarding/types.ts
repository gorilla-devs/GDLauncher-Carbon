import { JSX } from "solid-js"

export type OnboardingTrigger = "onClick" | "onMount" | "manual"

export interface OnboardingTipConfig {
  /** Unique identifier for this tip, stored in backend */
  id: string
  /** Title displayed at the top of the popover */
  title?: string
  /** Content to display in the popover body */
  description: string | JSX.Element
  /** When to trigger the tip */
  trigger: OnboardingTrigger
  /** Delay before showing (ms), useful for onMount trigger */
  delay?: number
  /** Popover placement relative to target element */
  placement?: "top" | "bottom" | "left" | "right"
  /** Whether this tip requires first launch to be completed first (default: true) */
  requiresFirstLaunchComplete?: boolean
}

export interface ActiveTipInfo {
  tipId: string
  targetRect: DOMRect
  config: OnboardingTipConfig
}

export interface OnboardingContextValue {
  /** Check if a tip has been seen */
  isTipSeen: (tipId: string) => boolean
  /** Mark a tip as seen (calls backend mutation) */
  markTipSeen: (tipId: string) => void
  /** Currently active tip (only one at a time) */
  activeTip: () => ActiveTipInfo | null
  /** Show a specific tip programmatically */
  showTip: (tipId: string, targetRect: DOMRect) => void
  /** Hide the current tip and mark as seen */
  hideTip: () => void
  /** All seen tip IDs (reactive) */
  seenTips: () => string[]
  /** Whether onboarding tips are enabled (false during first launch modal) */
  isEnabled: () => boolean
}
