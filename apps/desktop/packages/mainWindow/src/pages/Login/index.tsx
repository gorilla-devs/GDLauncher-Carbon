/**
 * Login Page Entry Point
 *
 * Redesigned auth flow with step-based architecture:
 *
 * Architecture:
 * - AuthFlow: Main orchestrator (AuthFlow.tsx)
 * - FlowController: State management and navigation (flow/FlowController.ts)
 * - AnimationController: First-class animation support (animations/AnimationController.ts)
 * - Step Components: WelcomeStep, TermsStep, AuthMethodStep, EnrollingStep, ProfileCreationStep, GdlAccountStep (steps/*)
 *
 * Flow:
 * 1. Welcome: Initial greeting
 * 2. Terms: Privacy and terms acceptance
 * 3. Auth Method: Choose browser OAuth or device code
 * 4. Enrolling: Browser/device code authentication with polling
 * 5. Profile Creation: For users without Minecraft profile
 * 6. GDL Account: Success + optional GDL account setup/linking
 *
 * Features:
 * - Promise-based flow control (no XState)
 * - Discriminated unions for type-safe state
 * - Global loading vs button loading patterns
 * - Real-time enrollment status polling
 * - Horizontal slide view transitions
 * - First-launch welcome animation
 * - Full TypeScript type safety
 */

import { createEffect, createMemo } from "solid-js"
import { useSearchParams } from "@solidjs/router"
import { AuthFlow } from "./AuthFlow"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"

export default function Login() {
  const globalStore = useGlobalStore()
  const [searchParams] = useSearchParams()
  const navigator = useGDNavigate()

  // Determine if we should redirect to library
  const shouldRedirect = createMemo(() => {
    // Wait for data to load
    const activeUuid = globalStore.currentlySelectedAccountUuid.data
    const settings = globalStore.settings.data
    const accounts = globalStore.accounts.data

    if (!settings || accounts === undefined || activeUuid === undefined) {
      return false // Data not loaded yet
    }

    // Don't redirect if explicitly adding accounts from settings
    const isAddingAccount =
      searchParams.addMicrosoftAccount === "true" ||
      searchParams.addGdlAccount === "true"

    if (isAddingAccount) {
      return false // Show auth flow
    }

    // Redirect if user has everything set up
    return (
      activeUuid !== null && // Has Microsoft account
      settings.termsAndPrivacyAccepted && // Accepted terms
      settings.gdlAccountId !== null // Has made decision about GDL (includes "" for skipped)
    )
  })

  // Perform redirect when conditions are met
  createEffect(() => {
    if (shouldRedirect()) {
      navigator.navigate("/library", { replace: true })
    }
  })

  return <AuthFlow />
}
