import {
  createContext,
  useContext,
  createSignal,
  createMemo,
  JSX
} from "solid-js"
import { rspc } from "@/utils/rspcClient"
import type {
  OnboardingContextValue,
  ActiveTipInfo,
  OnboardingTipConfig
} from "./types"

const OnboardingContext = createContext<OnboardingContextValue>()

// Registry for tip configs (registered by OnboardingTip components)
const tipRegistry = new Map<string, OnboardingTipConfig>()

export const registerTip = (id: string, config: OnboardingTipConfig) => {
  tipRegistry.set(id, config)
}

export const unregisterTip = (id: string) => {
  tipRegistry.delete(id)
}

export const getTipConfig = (id: string) => {
  return tipRegistry.get(id)
}

export const OnboardingProvider = (props: { children: JSX.Element }) => {
  // Query for seen tips from backend
  const seenTipsQuery = rspc.createQuery(() => ({
    queryKey: ["settings.getSeenOnboardingTips"]
  }))

  // Mutation to mark tip as seen
  const markSeenMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.markOnboardingTipSeen"]
  }))

  // Check first launch status
  const isFirstLaunch = rspc.createQuery(() => ({
    queryKey: ["settings.isFirstLaunch"]
  }))

  // Active tip state
  const [activeTipInfo, setActiveTipInfo] = createSignal<ActiveTipInfo | null>(
    null
  )

  // Derived state
  const seenTips = createMemo(() => seenTipsQuery.data ?? [])

  const isEnabled = createMemo(() => {
    // Disable during first launch onboarding modal
    return isFirstLaunch.data === false
  })

  const isTipSeen = (tipId: string): boolean => {
    return seenTips().includes(tipId)
  }

  const markTipSeen = (tipId: string) => {
    if (!isTipSeen(tipId)) {
      markSeenMutation.mutate(tipId)
    }
    // Also hide the tip if it's the active one
    if (activeTipInfo()?.tipId === tipId) {
      setActiveTipInfo(null)
    }
  }

  const showTip = (tipId: string, targetRect: DOMRect) => {
    const config = tipRegistry.get(tipId)
    if (!config) {
      console.warn(`Onboarding tip "${tipId}" not found in registry`)
      return
    }

    // Check if already seen
    if (isTipSeen(tipId)) {
      return
    }

    // Check first launch requirement
    const requiresFirstLaunch = config.requiresFirstLaunchComplete ?? true
    if (requiresFirstLaunch && isFirstLaunch.data === true) {
      return
    }

    // Check if tips are enabled
    if (!isEnabled()) {
      return
    }

    setActiveTipInfo({
      tipId,
      targetRect,
      config
    })
  }

  const hideTip = () => {
    const current = activeTipInfo()
    if (current) {
      markTipSeen(current.tipId)
    }
  }

  const contextValue: OnboardingContextValue = {
    isTipSeen,
    markTipSeen,
    activeTip: activeTipInfo,
    showTip,
    hideTip,
    seenTips,
    isEnabled
  }

  return (
    <OnboardingContext.Provider value={contextValue}>
      {props.children}
    </OnboardingContext.Provider>
  )
}

export const useOnboarding = (): OnboardingContextValue => {
  const context = useContext(OnboardingContext)
  if (!context) {
    throw new Error("useOnboarding must be used within OnboardingProvider")
  }
  return context
}
