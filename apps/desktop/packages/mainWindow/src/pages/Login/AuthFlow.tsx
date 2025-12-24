import {
  Switch,
  Match,
  createEffect,
  Show,
  onMount,
  onCleanup,
  Suspense,
  createSignal,
  createMemo,
  on
} from "solid-js"
import { useSearchParams } from "@solidjs/router"
import { Spinner, Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import { getCurrentOccasion } from "@/utils/occasions"
import { wideLogoUrl } from "@/utils/logos"
import BackgroundVideo from "/assets/images/login_background.webm"

import "./styles/viewTransitions.css"

import { FlowProvider, useFlow } from "./flow/FlowContext"
import { AnimationProvider, useAnimations } from "./animations/AnimationContext"
import { StepContainer } from "./components/StepContainer"
import type { AuthFlowConfig, AuthStep, AuthFlowState } from "./flow/types"
import { WelcomeStep } from "./steps/WelcomeStep"
import { TermsStep } from "./steps/TermsStep"
import { AuthMethodStep } from "./steps/AuthMethodStep"
import { EnrollingStep } from "./steps/EnrollingStep"
import { ProfileCreationStep } from "./steps/ProfileCreationStep"
import { GdlAccountStep } from "./steps/GdlAccountStep"
import { GdlAccountFormStep } from "./steps/GdlAccountFormStep"
import { GdlAccountVerificationStep } from "./steps/GdlAccountVerificationStep"
import { ErrorStep } from "./steps/ErrorStep"

/**
 * AuthFlow - Main Orchestrator
 *
 * Coordinates the entire authentication flow with:
 * - State management (FlowController)
 * - Animations (AnimationController)
 * - View transitions
 * - Loading states
 */

export function AuthFlow() {
  const globalStore = useGlobalStore()

  return (
    <Suspense
      fallback={
        <div class="flex h-screen w-full items-center justify-center">
          <Spinner class="h-10 w-10" />
        </div>
      }
    >
      <Show
        when={
          globalStore.accounts.data &&
          globalStore.settings.data &&
          globalStore.currentlySelectedAccountUuid.data !== undefined
        }
      >
        <AuthFlowInner />
      </Show>
    </Suspense>
  )
}

function AuthFlowInner() {
  const globalStore = useGlobalStore()
  const [searchParams] = useSearchParams()
  const rspcContext = rspc.useContext()

  // Extract configuration from global store
  const config: AuthFlowConfig = {
    activeUuid: globalStore.currentlySelectedAccountUuid.data!,
    accounts: globalStore.accounts.data!,
    isFirstLaunch: globalStore.settings.data!.isFirstLaunch,
    termsAndPrivacyAccepted: globalStore.settings.data!.termsAndPrivacyAccepted,
    gdlAccountId: globalStore.settings.data!.gdlAccountId,
    reducedMotion: globalStore.settings.data!.reducedMotion,
    isAddingMicrosoftFromSettings: searchParams.addMicrosoftAccount === "true",
    isAddingGdlFromSettings: searchParams.addGdlAccount === "true",
    shouldSetupGdlAfterAuth:
      searchParams.addMicrosoftAccount === "true" &&
      searchParams.addGdlAccount === "true",
    returnPath: searchParams.returnTo || null,
    currentOccasion: getCurrentOccasion()
  }

  // Create mutations
  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const saveGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.saveGdlAccount"]
  }))

  const enrollBeginMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.begin"]
  }))

  const enrollBeginBrowserMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.beginBrowser"]
  }))

  const enrollCancelMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.cancel"]
  }))

  const usernameAvailabilityMutation = rspc.createMutation(() => ({
    mutationKey: ["account.checkUsernameAvailable"]
  }))

  const createProfileMutation = rspc.createMutation(() => ({
    mutationKey: ["account.createProfile"]
  }))

  const enrollResumeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.resume"]
  }))

  // Create queries
  const accountsQuery = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }))

  const gdlAccountQuery = rspc.createQuery(() => ({
    queryKey: ["account.peekGdlAccount", config.activeUuid || ""],
    enabled: !!config.activeUuid
  }))

  return (
    <AnimationProvider reducedMotion={config.reducedMotion}>
      <FlowProvider
        config={config}
        rspcContext={rspcContext}
        settingsMutation={settingsMutation}
        saveGdlAccountMutation={saveGdlAccountMutation}
        enrollBeginMutation={enrollBeginMutation}
        enrollBeginBrowserMutation={enrollBeginBrowserMutation}
        enrollCancelMutation={enrollCancelMutation}
        usernameAvailabilityMutation={usernameAvailabilityMutation}
        createProfileMutation={createProfileMutation}
        enrollResumeMutation={enrollResumeMutation}
        accountsQuery={accountsQuery}
        gdlAccountQuery={gdlAccountQuery}
      >
        <AuthFlowContent />
      </FlowProvider>
    </AnimationProvider>
  )
}

function AuthFlowContent() {
  const flow = useFlow()
  const animations = useAnimations()
  const navigator = useGDNavigate()

  // Step-specific UI state
  const [termsAccepted, setTermsAccepted] = createSignal(false)
  const [username, setUsername] = createSignal("")
  const [usernameChecking, setUsernameChecking] = createSignal(false)
  const [usernameAvailable, setUsernameAvailable] = createSignal<
    "available" | "taken" | "notallowed" | null
  >(null)
  const [usernameError, setUsernameError] = createSignal<string | null>(null)
  const [buttonLoading, setButtonLoading] = createSignal(false)

  // GDL account form state
  const [gdlEmail, setGdlEmail] = createSignal("")
  const [gdlNickname, setGdlNickname] = createSignal("")
  const [gdlEmailError, setGdlEmailError] = createSignal<string | undefined>()
  const [gdlNicknameError, setGdlNicknameError] = createSignal<
    string | undefined
  >()
  const [gdlFormInitialized, setGdlFormInitialized] = createSignal(false)

  // Track if sidebar initial animation completed
  const [sidebarShown, setSidebarShown] = createSignal(false)

  // Username validation
  const isUsernameValid = createMemo(() => {
    const name = username()
    if (name.length === 0) return false
    if (name.length < 3 || name.length > 16) return false
    return /^[a-zA-Z0-9_]+$/.test(name)
  })

  const canSubmitProfile = createMemo(
    () => isUsernameValid() && usernameAvailable() === "available"
  )

  // Back button label (Cancel for enrolling/profile-creation, Back for others)
  const backButtonLabel = createMemo(() => {
    const step = getCurrentStep()
    if (!step) return "general:_trn_back"

    switch (step.type) {
      case "enrolling":
      case "profile-creation":
        return "general:_trn_cancel"
      default:
        return "general:_trn_back"
    }
  })

  // Debounced username availability check
  createEffect(() => {
    const name = username()
    const step = getCurrentStep()

    // Only check if on profile creation step
    if (step?.type !== "profile-creation") return

    // Reset states
    setUsernameAvailable(null)
    setUsernameError(null)

    if (!isUsernameValid()) return

    // Debounce the check
    const timer = setTimeout(async () => {
      setUsernameChecking(true)
      try {
        const result = await flow.checkUsernameAvailability(
          step.accessToken,
          name
        )
        setUsernameAvailable(result)
      } catch (err) {
        console.error("Error checking username:", err)
        setUsernameError("Failed to check username availability")
      } finally {
        setUsernameChecking(false)
      }
    }, 500)

    onCleanup(() => clearTimeout(timer))
  })

  // Initialize GDL form when entering gdl-account-form step
  createEffect(
    on(
      () => {
        const state = flow.state()
        if (state.phase === "content" && state.step.type === "gdl-account-form")
          return state.step
        return null
      },
      (step) => {
        if (step && !gdlFormInitialized()) {
          // Set default nickname from step (Microsoft username)
          if (step.nickname) {
            setGdlNickname(step.nickname)
          }
          // Set email if provided
          if (step.email) {
            setGdlEmail(step.email)
          }
          setGdlFormInitialized(true)
        } else if (!step) {
          // Reset when leaving the step
          setGdlFormInitialized(false)
        }
      }
    )
  )

  // Button handlers
  const handleWelcomeContinue = async () => {
    setButtonLoading(true)
    try {
      // Always go to terms step
      await flow.goToStep({ type: "terms", variant: "initial" })
    } finally {
      setButtonLoading(false)
    }
  }

  const handleTermsContinue = async () => {
    if (!termsAccepted()) return

    setButtonLoading(true)
    try {
      await flow.acceptTerms()

      const isAddingFromSettings =
        flow.data.isAddingMicrosoftFromSettings ||
        flow.data.isAddingGdlFromSettings

      if (isAddingFromSettings) {
        await flow.goToStep({ type: "auth-method" })
      } else {
        const hasActiveMicrosoftAccount = flow.data.accounts.some(
          (acc) => acc.uuid === flow.data.activeUuid
        )

        if (hasActiveMicrosoftAccount) {
          const gdlState = await flow.checkGDLAccount()

          if (gdlState.type === "none" || gdlState.type === "found-existing") {
            await flow.goToStep({ type: "gdl-account", gdlAccount: gdlState })
          } else {
            await flow.exitFlow("library", true)
          }
        } else {
          await flow.goToStep({ type: "auth-method" })
        }
      }
    } finally {
      setButtonLoading(false)
    }
  }

  const handleCreateProfile = async () => {
    if (!canSubmitProfile()) return

    const step = getCurrentStep()
    if (step?.type !== "profile-creation") return

    setButtonLoading(true)
    setUsernameError(null)

    try {
      await flow.createProfile(step.accessToken, username())
      const gdlState = await flow.checkGDLAccount()
      await flow.goToStep({ type: "gdl-account", gdlAccount: gdlState })
    } catch (err: any) {
      console.error("Error creating profile:", err)
      if (err.message?.includes("InvalidUsername")) {
        setUsernameError("Invalid username")
        setUsernameAvailable("notallowed")
      } else if (err.message?.includes("NameNotAvailable")) {
        setUsernameError("Username not available")
        setUsernameAvailable("taken")
      } else {
        setUsernameError("Failed to create profile")
      }
    } finally {
      setButtonLoading(false)
    }
  }

  const handleErrorRetry = async () => {
    setButtonLoading(true)
    try {
      // Restart initialization by reloading the page
      window.location.reload()
    } catch (error) {
      console.error("Failed to retry:", error)
      setButtonLoading(false)
    }
  }

  // GDL account form mutation
  const registerGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.registerGdlAccount"]
  }))

  const saveGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.saveGdlAccount"]
  }))

  // GDL account validation helpers
  const isGdlEmailValid = createMemo(() => {
    const email = gdlEmail().trim()
    if (email.length === 0) return false
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    return emailRegex.test(email)
  })

  const isGdlNicknameValid = createMemo(() => {
    const nickname = gdlNickname().trim()
    return nickname.length >= 3
  })

  const canSubmitGdlForm = createMemo(
    () => isGdlEmailValid() && isGdlNicknameValid()
  )

  const handleRegisterGdlAccount = async () => {
    if (!canSubmitGdlForm()) {
      // Show validation errors
      if (!gdlEmail().trim()) {
        setGdlEmailError("Email is required")
      } else if (!isGdlEmailValid()) {
        setGdlEmailError("Please enter a valid email address")
      }
      if (!gdlNickname().trim()) {
        setGdlNicknameError("Nickname is required")
      } else if (!isGdlNicknameValid()) {
        setGdlNicknameError("Nickname must be at least 3 characters")
      }
      return
    }

    setButtonLoading(true)
    setGdlEmailError(undefined)
    setGdlNicknameError(undefined)

    try {
      const activeUuid = flow.data.activeUuid
      if (!activeUuid) {
        throw new Error("No active account found")
      }

      // Register the account
      await registerGdlAccountMutation.mutateAsync({
        email: gdlEmail().trim(),
        nickname: gdlNickname().trim(),
        uuid: activeUuid
      })

      // Navigate to verification step
      await flow.goToStep({
        type: "gdl-account-verification",
        email: gdlEmail().trim(),
        uuid: activeUuid
      })
    } catch (error) {
      console.error("[AuthFlow] Failed to register GDL account:", error)
      setGdlEmailError(
        error instanceof Error ? error.message : "Failed to create account"
      )
    } finally {
      setButtonLoading(false)
    }
  }

  const handleVerifyLater = async () => {
    setButtonLoading(true)
    try {
      const step = getCurrentStep()
      if (step?.type === "gdl-account-verification") {
        // Save the UUID even though not verified
        await saveGdlAccountMutation.mutateAsync(step.uuid)
      }
      await flow.exitFlow("library", flow.data.isFirstLaunch)
    } catch (error) {
      console.error("[AuthFlow] Failed to verify later:", error)
      setButtonLoading(false)
    }
  }

  // GDL Account step handlers
  const handleSyncExistingAccount = async () => {
    const step = getCurrentStep()
    if (step?.type !== "gdl-account") return

    const gdlAccount = step.gdlAccount
    if (gdlAccount?.type !== "found-existing") return

    setButtonLoading(true)
    try {
      await flow.linkExistingGDLAccount(gdlAccount.data)
      await flow.exitFlow("library", flow.data.isFirstLaunch)
    } catch (error) {
      console.error("[AuthFlow] Failed to link GDL account:", error)
      setButtonLoading(false)
    }
  }

  const handleSetupGDLAccount = async () => {
    // Get default nickname from current account
    const account = flow.data.accounts.find(
      (acc) => acc.uuid === flow.data.activeUuid
    )
    const defaultNickname = account?.username || ""

    await flow.goToStep({
      type: "gdl-account-form",
      email: "",
      nickname: defaultNickname
    })
  }

  const handleGdlContinue = async () => {
    setButtonLoading(true)
    try {
      await flow.exitFlow("library", flow.data.isFirstLaunch)
    } finally {
      setButtonLoading(false)
    }
  }

  const handleRetryGdlCheck = async () => {
    setButtonLoading(true)
    try {
      const gdlState = await flow.checkGDLAccount(true)
      await flow.goToStep({ type: "gdl-account", gdlAccount: gdlState })
    } catch (error) {
      console.error("[AuthFlow] Failed to retry GDL check:", error)
    } finally {
      setButtonLoading(false)
    }
  }

  // DOM refs
  let sidebarRef: HTMLDivElement | undefined
  let videoRef: HTMLVideoElement | undefined
  let stepContainerRef: HTMLDivElement | undefined
  let backgroundBlurRef: HTMLDivElement | undefined
  let loadingSpinnerRef: HTMLDivElement | undefined
  let backButtonRef: HTMLDivElement | undefined
  let skipButtonRef: HTMLDivElement | undefined
  let welcomeToTextRef: HTMLDivElement | undefined
  let gdlauncherTextRef: HTMLDivElement | undefined

  // Register refs when mounted
  onMount(() => {
    animations.registerRefs({
      sidebar: sidebarRef,
      video: videoRef,
      stepContainer: stepContainerRef,
      backgroundBlur: backgroundBlurRef,
      loadingSpinner: loadingSpinnerRef,
      backButton: backButtonRef,
      skipButton: skipButtonRef,
      welcomeToText: welcomeToTextRef,
      gdlauncherText: gdlauncherTextRef
    })

    // Back button visibility will be handled by createEffect
  })

  // Cleanup on unmount
  onCleanup(() => {
    flow.cleanup()
  })

  // Handle state transitions
  createEffect(() => {
    const state = flow.state()

    // Loading → Content: animate sidebar in (ONLY ONCE on initial load)
    if (state.phase === "content" && !sidebarShown()) {
      animations.sidebar.slideIn()
      setSidebarShown(true)
    }

    // NOTE: Step transitions are now handled by View Transition API in FlowController

    // Exiting: handle exit animations
    if (state.phase === "exiting") {
      handleExit(state.destination, state.showSeasonal)
    }
  })

  // Handle back button visibility changes (step-based)
  createEffect(
    on(
      () => {
        const step = getCurrentStepForEffects()
        if (!step) return null

        switch (step.type) {
          case "welcome":
            return false
          case "terms":
            return step.variant !== "forced"
          case "auth-method":
          case "enrolling":
          case "profile-creation":
          case "gdl-account-form":
            return true
          case "error":
          case "gdl-account-verification":
            return false
          case "gdl-account":
            // Show back button when adding from settings (allows cancel)
            return flow.data.isAddingGdlFromSettings
          default:
            return false
        }
      },
      (shouldShowBack) => {
        if (shouldShowBack === null) return

        if (shouldShowBack) {
          animations.backButton.show()
        } else {
          animations.backButton.hide()
        }
      }
    )
  )

  // Handle skip button visibility changes
  createEffect(
    on(
      () => {
        const step = getCurrentStepForEffects()
        const isAddingFromSettings =
          flow.data.isAddingMicrosoftFromSettings ||
          flow.data.isAddingGdlFromSettings

        // Show skip button when:
        // 1. On gdl-account step AND NOT adding from settings AND user hasn't made a decision yet
        // 2. On gdl-account-verification step (as "Verify Later")
        const shouldShowSkip =
          (!isAddingFromSettings &&
            step?.type === "gdl-account" &&
            flow.data.gdlAccountId === null) ||
          step?.type === "gdl-account-verification"

        return shouldShowSkip
      },
      (shouldShowSkip) => {
        if (shouldShowSkip) {
          animations.skipButton.show()
        } else {
          animations.skipButton.hide()
        }
      }
    )
  )

  // Handle exit flow
  const handleExit = async (
    destination: "library" | "settings",
    showSeasonal: boolean
  ) => {
    // Slide sidebar out
    await animations.sidebar.slideOut()

    // Show seasonal splash if applicable
    if (showSeasonal && flow.data.currentOccasion) {
      await animations.seasonal.show(flow.data.currentOccasion)
      await new Promise((resolve) => setTimeout(resolve, 3000))
      await animations.seasonal.hide()
    }

    // First launch welcome animation
    if (flow.data.isFirstLaunch) {
      await animations.text.fadeIn("welcomeToText", { duration: 500 })
      await new Promise((resolve) => setTimeout(resolve, 800))

      await animations.text.fadeIn("gdlauncherText", { duration: 600 })
      await new Promise((resolve) => setTimeout(resolve, 1200))

      await animations.text.fadeOut(["welcomeToText", "gdlauncherText"], {
        duration: 500
      })
    }

    // Navigate
    if (destination === "settings" && flow.data.returnPath) {
      navigator.navigate(flow.data.returnPath)
    } else {
      navigator.navigate("/library")
    }
  }

  // Get step title
  const getStepTitle = () => {
    const state = flow.state()
    if (state.phase !== "content" && state.phase !== "transitioning") return ""

    const step = state.phase === "content" ? state.step : state.to

    switch (step.type) {
      case "welcome":
        return <Trans key="auth:_trn_login.titles.welcome_to_gdlauncher" />
      case "terms":
        return step.variant === "forced" ? (
          <Trans key="auth:_trn_login.titles.updated_terms" />
        ) : (
          <Trans key="auth:_trn_login.titles.terms_and_privacy" />
        )
      case "auth-method":
        return <Trans key="auth:_trn_login.titles.sign_in_with_microsoft" />
      case "enrolling":
        return step.method === "browser" ? (
          <Trans key="auth:_trn_login.titles.browser_authentication" />
        ) : (
          <Trans key="auth:_trn_login.titles.microsoft_code_step" />
        )
      case "profile-creation":
        return <Trans key="auth:_trn_login.titles.create_profile" />
      case "gdl-account":
        return <Trans key="auth:_trn_login.enable_cloud_sync" />
      case "gdl-account-form":
        return <Trans key="auth:_trn_login.titles.create_gdl_account" />
      case "gdl-account-verification":
        return <Trans key="auth:_trn_login.titles.gdl_account_verification" />
      case "error":
        return <Trans key="auth:_trn_login.titles.something_went_wrong" />
      default:
        return ""
    }
  }

  // Get video source based on occasion
  const videoSource =
    flow.data.currentOccasion?.assets.authVideo ?? BackgroundVideo

  // Helper to get current step for RENDERING (shows new step during transitions)
  function getCurrentStep() {
    const state = flow.state()
    if (state.phase === "content") return state.step
    // During transitions, render the NEW step (to) for view transitions
    if (state.phase === "transitioning") return state.to
    return null
  }

  // Helper to get current step for EFFECTS (prevents double-triggering)
  function getCurrentStepForEffects() {
    const state = flow.state()
    if (state.phase === "content") return state.step
    // During transitions, keep tracking the FROM step for effects
    if (state.phase === "transitioning") return state.from
    return null
  }

  // Helper to get typed step when it matches
  const getStepAs = <T extends AuthStep["type"]>(type: T) => {
    const step = getCurrentStep()
    return step?.type === type ? (step as Extract<AuthStep, { type: T }>) : null
  }

  return (
    <div class="flex h-screen w-full" id="auth-flow">
      {/* Sidebar - always rendered for animation refs */}
      <div
        ref={sidebarRef}
        class="text-lightSlate-50 bg-darkSlate-800 z-1 absolute z-10 flex h-full flex-col items-center overflow-hidden rounded-md -translate-x-full"
        style={{
          width: "35%",
          "min-width": "400px",
          "max-width": "600px"
        }}
      >
        {/* Logo */}
        <div class="h-30 flex justify-center">
          <img class="w-60" src={wideLogoUrl} alt="GDLauncher" />
        </div>

        <Show
          when={
            flow.state().phase === "content" ||
            flow.state().phase === "transitioning"
          }
        >
          {/* Title */}
          <div class="mb-4 flex items-center justify-center gap-2 text-lg font-bold">
            {getStepTitle()}
          </div>

          {/* Step content */}
          <div class="box-border relative w-full flex-1 overflow-hidden px-4 min-h-0">
            <StepContainer
              ref={(el) => (stepContainerRef = el)}
              direction={
                flow.nextDirection() ||
                (flow.state().phase === "transitioning"
                  ? (
                      flow.state() as Extract<
                        AuthFlowState,
                        { phase: "transitioning" }
                      >
                    ).direction
                  : undefined)
              }
            >
              <Switch>
                <Match when={getCurrentStep()?.type === "welcome"}>
                  <WelcomeStep />
                </Match>

                <Match when={getStepAs("terms")}>
                  {(step) => (
                    <TermsStep
                      step={step()}
                      termsAccepted={termsAccepted()}
                      onTermsAcceptedChange={setTermsAccepted}
                    />
                  )}
                </Match>

                <Match when={getCurrentStep()?.type === "auth-method"}>
                  <AuthMethodStep />
                </Match>

                <Match when={getStepAs("enrolling")}>
                  {(step) => <EnrollingStep step={step()} />}
                </Match>

                <Match when={getStepAs("profile-creation")}>
                  {(step) => (
                    <ProfileCreationStep
                      step={step()}
                      username={username()}
                      onUsernameChange={setUsername}
                      checking={usernameChecking()}
                      available={usernameAvailable()}
                      error={usernameError()}
                    />
                  )}
                </Match>

                <Match when={getStepAs("gdl-account")}>
                  {(step) => <GdlAccountStep step={step()} />}
                </Match>

                <Match when={getStepAs("gdl-account-form")}>
                  {(step) => (
                    <GdlAccountFormStep
                      step={step()}
                      email={gdlEmail()}
                      nickname={gdlNickname()}
                      onEmailChange={setGdlEmail}
                      onNicknameChange={setGdlNickname}
                      emailError={gdlEmailError()}
                      nicknameError={gdlNicknameError()}
                    />
                  )}
                </Match>

                <Match when={getStepAs("gdl-account-verification")}>
                  {(step) => <GdlAccountVerificationStep step={step()} />}
                </Match>

                <Match when={getStepAs("error")}>
                  {(step) => <ErrorStep step={step()} />}
                </Match>
              </Switch>
            </StepContainer>
          </div>
        </Show>

        {/* Footer with buttons - always rendered, visibility controlled by animation */}
        <div class="box-border flex w-full flex-col items-center p-4">
          <div class="box-border flex w-full gap-2">
            {/* Back button */}
            <div
              ref={backButtonRef}
              class="overflow-hidden"
              style={{
                width: "0",
                margin: "0"
              }}
            >
              <Button
                size="large"
                type="secondary"
                fullWidth
                onClick={() => {
                  const step = getCurrentStep()
                  if (!step) return

                  const isAddingFromSettings =
                    flow.data.isAddingMicrosoftFromSettings ||
                    flow.data.isAddingGdlFromSettings

                  // Manual backward navigation with explicit direction
                  switch (step.type) {
                    case "terms":
                      flow.goToStep(
                        { type: "welcome" },
                        { direction: "backward" }
                      )
                      break
                    case "auth-method":
                      if (isAddingFromSettings) {
                        flow.exitFlow("settings")
                      } else {
                        flow.goToStep(
                          { type: "terms", variant: "initial" },
                          { direction: "backward" }
                        )
                      }
                      break
                    case "enrolling":
                      flow.goToStep(
                        { type: "auth-method" },
                        { direction: "backward" }
                      )
                      break
                    case "profile-creation":
                      // Cancel enrollment and return to auth-method
                      flow.cancelEnrollment()
                      flow.goToStep(
                        { type: "auth-method" },
                        { direction: "backward" }
                      )
                      break
                    case "gdl-account":
                      // When adding from settings, return to settings
                      if (isAddingFromSettings) {
                        flow.exitFlow("settings")
                      }
                      break
                    case "gdl-account-form":
                      // Go back to gdl-account info step
                      flow.goToStep(
                        { type: "gdl-account", gdlAccount: { type: "none" } },
                        { direction: "backward" }
                      )
                      break
                  }
                }}
              >
                <div class="i-hugeicons:arrow-left-01" />
                <Trans key={backButtonLabel()} />
              </Button>
            </div>

            {/* Skip to library button */}
            <div
              ref={skipButtonRef}
              class="overflow-hidden"
              style={{
                width: "0",
                margin: "0"
              }}
            >
              <Button
                size="large"
                type="secondary"
                fullWidth
                onClick={async () => {
                  const step = getCurrentStep()
                  if (step?.type === "gdl-account") {
                    // Save the skip decision
                    await flow.skipGDLAccount()
                    flow.exitFlow("library", flow.data.isFirstLaunch)
                  } else if (step?.type === "gdl-account-verification") {
                    // Verify later - save UUID and exit
                    await handleVerifyLater()
                  } else {
                    // Default skip
                    flow.exitFlow("library", flow.data.isFirstLaunch)
                  }
                }}
                loading={buttonLoading()}
                disabled={buttonLoading()}
              >
                <Show
                  when={getCurrentStep()?.type === "gdl-account-verification"}
                  fallback={
                    <>
                      <Trans key="auth:_trn_login.skip_for_now" />
                      <div class="i-hugeicons:arrow-right-01" />
                    </>
                  }
                >
                  <Trans key="auth:_trn_login.verify_later" />
                  <div class="i-hugeicons:arrow-right-01" />
                </Show>
              </Button>
            </div>

            {/* Step-specific action buttons */}
            {/* Welcome Step - Continue button */}
            <Show when={getCurrentStep()?.type === "welcome"}>
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleWelcomeContinue}
                loading={buttonLoading()}
                disabled={buttonLoading()}
              >
                <Trans key="general:_trn_continue" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* Terms Step - Continue button */}
            <Show when={getCurrentStep()?.type === "terms"}>
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleTermsContinue}
                disabled={!termsAccepted() || buttonLoading()}
                loading={buttonLoading()}
              >
                <Trans key="general:_trn_continue" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* Profile Creation Step - Create Profile button */}
            <Show when={getCurrentStep()?.type === "profile-creation"}>
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleCreateProfile}
                disabled={!canSubmitProfile() || buttonLoading()}
                loading={buttonLoading()}
              >
                <Trans key="auth:_trn_profile_creation.create_profile" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* Error Step - Retry button */}
            <Show when={getStepAs("error")}>
              {(step) => (
                <Show when={step().canRetry}>
                  <Button
                    size="large"
                    type="primary"
                    fullWidth
                    onClick={handleErrorRetry}
                    loading={buttonLoading()}
                    disabled={buttonLoading()}
                  >
                    <Trans key="general:_trn_retry" />
                    <div class="i-hugeicons:refresh h-4 w-4" />
                  </Button>
                </Show>
              )}
            </Show>

            {/* GDL Account Form Step - Register button */}
            <Show when={getCurrentStep()?.type === "gdl-account-form"}>
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleRegisterGdlAccount}
                disabled={!canSubmitGdlForm() || buttonLoading()}
                loading={buttonLoading()}
              >
                <Trans key="auth:_trn_login.register" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* GDL Account Step - Sync existing account button */}
            <Show
              when={
                getStepAs("gdl-account")?.gdlAccount?.type === "found-existing"
              }
            >
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleSyncExistingAccount}
                loading={buttonLoading()}
                disabled={buttonLoading()}
              >
                <Trans key="auth:_trn_login.sync_account" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* GDL Account Step - Enable Cloud Sync button (no existing account) */}
            <Show
              when={
                getStepAs("gdl-account")?.gdlAccount?.type === "none" &&
                flow.data.gdlAccountId !== ""
              }
            >
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleSetupGDLAccount}
                loading={buttonLoading()}
                disabled={buttonLoading()}
              >
                <Trans key="auth:_trn_login.enable_cloud_sync" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* GDL Account Step - Continue button (already linked or skipped) */}
            <Show
              when={
                getStepAs("gdl-account")?.gdlAccount?.type === "linked" ||
                (getStepAs("gdl-account")?.gdlAccount?.type === "none" &&
                  flow.data.gdlAccountId === "")
              }
            >
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleGdlContinue}
                loading={buttonLoading()}
                disabled={buttonLoading()}
              >
                <Trans key="general:_trn_continue" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
              </Button>
            </Show>

            {/* GDL Account Step - Retry button (error state) */}
            <Show when={getStepAs("gdl-account")?.gdlAccount?.type === "error"}>
              <Button
                size="large"
                type="primary"
                fullWidth
                onClick={handleRetryGdlCheck}
                loading={buttonLoading()}
                disabled={buttonLoading()}
              >
                <Trans key="general:_trn_retry" />
                <div class="i-hugeicons:refresh h-4 w-4" />
              </Button>
            </Show>
          </div>
        </div>
      </div>

      {/* Background video */}
      <div class="w-full flex-1">
        <div
          ref={backgroundBlurRef}
          class="z-1 absolute left-0 top-0 h-screen w-full bg-black/20 p-0"
          style={{
            "backdrop-filter": "blur(6px)"
          }}
        />
        <div class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 text-7xl font-bold leading-loose">
          <div ref={welcomeToTextRef} class="opacity-0">
            <Trans key="auth:_trn_login.welcome_to" />
          </div>
          <div ref={gdlauncherTextRef} class="opacity-0">
            <Trans key="auth:_trn_login.gdlauncher" />
          </div>
        </div>
        <div
          class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 transition-opacity duration-300"
          classList={{
            "opacity-100": flow.state().phase === "loading",
            "opacity-0 pointer-events-none": flow.state().phase !== "loading"
          }}
        >
          <div ref={loadingSpinnerRef}>
            <Spinner class="h-10 w-10" />
          </div>
        </div>
        <video
          ref={videoRef}
          class="h-screen w-full object-cover p-0"
          src={videoSource}
          autoplay
          muted
          loop
          playsinline
        />
      </div>
    </div>
  )
}
