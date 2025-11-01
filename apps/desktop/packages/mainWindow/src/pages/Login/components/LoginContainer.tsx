import { Button, Spinner } from "@gd/ui"
import {
  Switch,
  Match,
  createEffect,
  createSignal,
  onCleanup,
  Show
} from "solid-js"
import { Trans } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"
import Logo from "/assets/images/gdlauncher_wide_logo_blue.svg"
import BackgroundVideo from "/assets/images/login_background.webm"
import { useSearchParams } from "@solidjs/router"
import { useMachine } from "@xstate/solid"

// Import step components
import { ProgressStepper } from "./ProgressStepper"
import { WelcomeStep } from "./WelcomeStep"
import { TermsAndPrivacyStep } from "./TermsAndPrivacyStep"
import { AuthMethodStep } from "./AuthMethodStep"
import { BrowserAuthStep } from "./BrowserAuthStep"
import { DeviceCodeStepEnhanced } from "./DeviceCodeStepEnhanced"
import { CompleteStep } from "./CompleteStep"
import { GDLAccountSetupModal } from "./GDLAccountSetupModal"
import ProfileCreationStep from "../ProfileCreationStep"

// Import occasion utilities
import { getCurrentOccasion } from "@/utils/occasions"

// Import login utilities
import { handleStatus } from "@/utils/login"

// Import XState machine
import { createLoginMachine } from "../machines/loginMachine"
import { createServices } from "../machines/loginMachine.services"
import { type LoginMachineEvents } from "../machines/loginMachine.types"

// Import styles
import "../styles/viewTransitions.css"

/**
 * Main Login Container Component - XState Version
 *
 * Uses a state machine to manage the authentication flow
 */

export function LoginContainer() {
  const globalStore = useGlobalStore()
  const navigator = useGDNavigate()
  const rspcContext = rspc.useContext()
  const [searchParams] = useSearchParams()

  // Mutations
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

  const enrollFinalizeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.finalize"]
  }))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  // Enrollment status polling control
  const [isPollingEnabled, setIsPollingEnabled] = createSignal(false)

  // Track back button visibility to prevent duplicate animations
  const [isBackButtonShown, setIsBackButtonShown] = createSignal(false)

  // Enrollment status query - polls during active enrollment
  const enrollmentStatusQuery = rspc.createQuery(() => ({
    queryKey: ["account.enroll.getStatus"],
    refetchInterval: 500, // Poll every 500ms during enrollment
    enabled: isPollingEnabled() // Controlled by state machine state
  }))

  // Create services and actions with dependencies
  const servicesAndActions = createServices({
    rspcContext,
    globalStore,
    saveGdlAccountMutation,
    enrollBeginMutation,
    enrollBeginBrowserMutation,
    enrollCancelMutation,
    enrollFinalizeMutation,
    settingsMutation,
    navigator
  })

  // Create and start machine
  const loginMachine = createLoginMachine(servicesAndActions)

  const [state, send] = useMachine(loginMachine, {
    input: {
      isAddingMicrosoftAccount: searchParams.addMicrosoftAccount === "true",
      isAddingGdlAccount: searchParams.addGdlAccount === "true",
      isAddingAccount:
        searchParams.addMicrosoftAccount === "true" ||
        searchParams.addGdlAccount === "true",
      returnPath: searchParams.returnTo || null,
      currentOccasion: getCurrentOccasion(),
      reducedMotion: globalStore.settings.data?.reducedMotion ?? false
    }
  })

  // Debug logging
  console.log("[LoginContainer] Query params:", {
    addMicrosoftAccount: searchParams.addMicrosoftAccount,
    addGdlAccount: searchParams.addGdlAccount,
    returnTo: searchParams.returnTo,
    isAddingAccount: state.context.isAddingAccount,
    currentState: state.value
  })

  // DOM refs
  let sidebarRef: HTMLDivElement | undefined
  let backgroundBlurRef: HTMLDivElement | undefined
  let loadingSpinnerRef: HTMLDivElement | undefined
  let videoRef: HTMLVideoElement | undefined
  let btnRef: HTMLDivElement | undefined
  let welcomeToTextRef: HTMLDivElement | undefined
  let gdlauncherTextRef: HTMLDivElement | undefined

  // Update refs in machine context
  createEffect(() => {
    send({
      type: "UPDATE_REFS",
      refs: {
        sidebar: sidebarRef,
        video: videoRef,
        backgroundBlur: backgroundBlurRef,
        loadingSpinner: loadingSpinnerRef,
        backButton: btnRef,
        welcomeToText: welcomeToTextRef,
        gdlauncherText: gdlauncherTextRef
      }
    })
  })

  // Animate back button visibility based on state changes with deduplication
  createEffect(() => {
    if (!btnRef) return

    const shouldShow = isBackButtonVisible()

    // Only animate if visibility actually changed (prevents double animation)
    if (shouldShow !== isBackButtonShown()) {
      setIsBackButtonShown(shouldShow)

      if (shouldShow) {
        btnRef.animate(
          [
            { width: "0", margin: "0" },
            { width: "60%", margin: "0 1rem 0 0" }
          ],
          {
            duration: 300,
            easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
            fill: "forwards"
          }
        )
      } else {
        btnRef.animate(
          [
            { width: "60%", margin: "0 1rem 0 0" },
            { width: "0", margin: "0" }
          ],
          {
            duration: 300,
            easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
            fill: "forwards"
          }
        )
      }
    }
  })

  // Enable enrollment status polling when in enrollment states
  createEffect(() => {
    const isEnrolling =
      state.matches({ authFlow: { enrolling: "waitingForBrowser" } }) ||
      state.matches({ authFlow: { enrolling: "pollingCode" } })

    setIsPollingEnabled(isEnrolling)
  })

  // Monitor enrollment status and send events to state machine
  createEffect(() => {
    if (!enrollmentStatusQuery.data || !isPollingEnabled()) return

    // Handle browser auth status
    if (state.matches({ authFlow: { enrolling: "waitingForBrowser" } })) {
      handleStatus(enrollmentStatusQuery, {
        onWaitingForBrowser: (info) => {
          send({
            type: "BROWSER_AUTH_READY",
            data: {
              authUrl: info.authUrl,
              redirectUri: info.redirectUri,
              expiresAt:
                typeof info.expiresAt === "string"
                  ? parseInt(info.expiresAt, 10)
                  : info.expiresAt
            }
          })
        },
        onComplete: () => {
          send({ type: "AUTH_COMPLETE" })
        },
        onNeedsProfileCreation: (accessToken) => {
          send({
            type: "PROFILE_CREATION_REQUIRED",
            data: accessToken
          })
        },
        onFail: (error) => {
          send({
            type: "AUTH_ERROR",
            error
          })
        }
      })
    }

    // Handle device code auth status
    if (state.matches({ authFlow: { enrolling: "pollingCode" } })) {
      handleStatus(enrollmentStatusQuery, {
        onPolling: (deviceCode) => {
          send({
            type: "POLLING_CODE_RECEIVED",
            data: {
              userCode: deviceCode.userCode,
              link: deviceCode.verificationUri,
              verificationUri: deviceCode.verificationUri,
              expiresAt: deviceCode.expiresAt
            }
          })
        },
        onComplete: () => {
          send({ type: "AUTH_COMPLETE" })
        },
        onNeedsProfileCreation: (accessToken) => {
          send({
            type: "PROFILE_CREATION_REQUIRED",
            data: accessToken
          })
        },
        onFail: (error) => {
          send({
            type: "AUTH_ERROR",
            error
          })
        }
      })
    }
  })

  // Cleanup: Disable polling on unmount
  onCleanup(() => {
    setIsPollingEnabled(false)
  })

  // Helper function to send events with view transitions
  const sendWithTransition = (event: LoginMachineEvents) => {
    const shouldTransition =
      !globalStore.settings.data?.reducedMotion &&
      typeof document !== "undefined" &&
      "startViewTransition" in document

    if (shouldTransition) {
      document.startViewTransition(() => {
        send(event)
      })
    } else {
      send(event)
    }
  }

  // Determine video source based on occasion
  const videoSource =
    state.context.currentOccasion?.assets.authVideo ?? BackgroundVideo

  // Get step title based on current state
  const getStepTitle = () => {
    // @ts-expect-error - XState v5 state.matches() has type inference issues
    if (state.matches("onboarding.welcome"))
      return <Trans key="login.titles.welcome_to_gdlauncher" />
    // @ts-expect-error - XState v5 state.matches() has type inference issues
    if (state.matches("onboarding.termsAndPrivacy"))
      return <Trans key="login.titles.terms_and_privacy" />
    // @ts-expect-error - XState v5 state.matches() has type inference issues
    if (state.matches("authFlow.authMethod"))
      return <Trans key="login.titles.sign_in_with_microsoft" />
    if (
      state.matches({ authFlow: { enrolling: "browser" } }) ||
      state.matches({ authFlow: { enrolling: "waitingForBrowser" } })
    )
      return <Trans key="login.titles.browser_authentication" />
    if (
      state.matches({ authFlow: { enrolling: "deviceCode" } }) ||
      state.matches({ authFlow: { enrolling: "pollingCode" } })
    )
      return <Trans key="login.titles.microsoft_code_step" />
    if (state.matches({ authFlow: { enrolling: "profileCreation" } }))
      return <Trans key="login.titles.create_profile" />
    if (state.matches("completingAuth"))
      return <Trans key="login.titles.authentication_complete" />
    if (state.matches("checkingGDLAccount"))
      return <Trans key="login.titles.gdl_account_verification" />
    if (state.matches("complete")) return <Trans key="login.titles.all_set" />
    return ""
  }

  const isBackButtonVisible = () => {
    // Blacklist: Only show back button in these specific interactive states
    return (
      // @ts-expect-error - XState v5 state.matches() has type inference issues
      state.matches("onboarding.termsAndPrivacy") ||
      // @ts-expect-error - XState v5 state.matches() has type inference issues
      state.matches("authFlow.authMethod") ||
      state.matches({ authFlow: { enrolling: "waitingForBrowser" } }) ||
      state.matches({ authFlow: { enrolling: "pollingCode" } }) ||
      state.matches({ authFlow: { enrolling: "profileCreation" } }) ||
      state.matches({ authFlow: { enrolling: "failed" } }) ||
      (state.matches("complete") && state.context.isAddingAccount)
    )
  }

  return (
    <>
      {/* Seasonal Content Overlay */}
      <Show when={state.matches("seasonalSplash")}>
        <>
          {/* Dark Overlay */}
          <div class="absolute inset-0 bg-black/30 z-40" />

          {/* Seasonal Content */}
          <div class="absolute inset-0 z-50 flex flex-col items-center justify-center">
            {/* Seasonal Message */}
            <div
              class="mb-8 text-center transition-all duration-1000 ease-out"
              classList={{
                "opacity-0 translate-y-5":
                  !state.context.seasonalMessageVisible,
                "opacity-100 translate-y-0":
                  state.context.seasonalMessageVisible
              }}
            >
              <h1
                class="text-7xl font-bold leading-tight"
                style={{
                  color: state.context.currentOccasion!.colors.primary,
                  "text-shadow": `0 0 40px ${state.context.currentOccasion!.colors.accent}, 0 4px 20px rgba(0, 0, 0, 0.5)`
                }}
              >
                {state.context.currentOccasion!.message}
              </h1>
            </div>

            {/* Continue Button */}
            <Show when={state.context.seasonalButtonVisible}>
              <div
                class="transition-all duration-500 ease-out"
                classList={{
                  "opacity-0 translate-y-2.5":
                    !state.context.seasonalButtonVisible,
                  "opacity-100 translate-y-0":
                    state.context.seasonalButtonVisible
                }}
              >
                <Button
                  size="large"
                  variant="primary"
                  onClick={() => send({ type: "CONTINUE_SEASONAL" })}
                  style={{
                    "background-color":
                      state.context.currentOccasion!.colors.primary,
                    "border-color": state.context.currentOccasion!.colors.accent
                  }}
                >
                  Continue to Library
                  <div class="i-hugeicons:arrow-right-01 ml-2" />
                </Button>
              </div>
            </Show>
          </div>
        </>
      </Show>

      <div class="flex h-screen w-full" id="main-login-page">
        {/* Sidebar */}
        <Show when={!state.matches("seasonalSplash")}>
          <div
            ref={sidebarRef}
            class="text-lightSlate-50 bg-darkSlate-800 z-1 absolute z-10 flex h-full -translate-x-full flex-col items-center overflow-hidden rounded-md"
            style={{
              width: "35%",
              "min-width": "400px",
              "max-width": "600px"
            }}
          >
            {/* Logo */}
            <div class="h-30 flex justify-center">
              <img class="w-60" src={Logo} alt="GDLauncher" />
            </div>

            {/* Title */}
            <div class="mb-4 flex items-center justify-center gap-2 text-lg font-bold">
              {getStepTitle()}
            </div>

            {/* Step content */}
            <div class="box-border relative h-auto w-full flex-1 overflow-hidden px-4">
              <div
                class="flex h-full w-full justify-center overflow-y-auto overflow-x-hidden"
                style={{
                  "view-transition-name": "step-content",
                  // @ts-expect-error - view-transition-class not in TypeScript types yet
                  "view-transition-class": state.context.transitionDirection
                }}
              >
                <Switch>
                  {/* @ts-expect-error - XState v5 state.matches() has type inference issues */}
                  <Match when={state.matches("onboarding.welcome")}>
                    <WelcomeStep
                      hasActiveAccount={!!state.context.activeUuid}
                    />
                  </Match>

                  {/* @ts-expect-error - XState v5 state.matches() has type inference issues */}
                  <Match when={state.matches("onboarding.termsAndPrivacy")}>
                    <TermsAndPrivacyStep
                      initialAccepted={state.context.termsAccepted}
                      onAcceptanceChange={(accepted) =>
                        send({
                          type: accepted ? "ACCEPT_TERMS" : "ACCEPT_TERMS"
                        })
                      }
                    />
                  </Match>

                  {/* @ts-expect-error - XState v5 state.matches() has type inference issues */}
                  <Match when={state.matches("authFlow.authMethod")}>
                    <AuthMethodStep
                      onBrowserAuth={() =>
                        sendWithTransition({ type: "SELECT_BROWSER_AUTH" })
                      }
                      onDeviceCodeAuth={() =>
                        sendWithTransition({ type: "SELECT_DEVICE_CODE" })
                      }
                      loading={state.context.loadingButton}
                    />
                  </Match>

                  <Match
                    when={state.matches({
                      authFlow: { enrolling: "waitingForBrowser" }
                    })}
                  >
                    <BrowserAuthStep
                      authUrl={
                        (enrollmentStatusQuery.data as any)?.waitingForBrowser
                          ?.auth_url
                      }
                      redirectUri={
                        (enrollmentStatusQuery.data as any)?.waitingForBrowser
                          ?.redirect_uri
                      }
                      expiresAt={
                        (enrollmentStatusQuery.data as any)?.waitingForBrowser
                          ?.expires_at
                      }
                      currentStage={
                        typeof enrollmentStatusQuery.data === "string"
                          ? enrollmentStatusQuery.data
                          : (enrollmentStatusQuery.data as any)
                                ?.waitingForBrowser
                            ? "waitingForBrowser"
                            : "waiting"
                      }
                      isEnrolling={true}
                      onSwitchToDeviceCode={() =>
                        sendWithTransition({ type: "SWITCH_TO_DEVICE_CODE" })
                      }
                      onCancel={() => sendWithTransition({ type: "BACK" })}
                      onRetry={() =>
                        sendWithTransition({ type: "SELECT_BROWSER_AUTH" })
                      }
                    />
                  </Match>

                  <Match
                    when={state.matches({
                      authFlow: { enrolling: "pollingCode" }
                    })}
                  >
                    <DeviceCodeStepEnhanced
                      deviceCodeObject={state.context.deviceCodeObject}
                      setDeviceCodeObject={() => {}}
                      nextStep={() => {}}
                      prevStep={() => sendWithTransition({ type: "BACK" })}
                      enrollmentStatus={enrollmentStatusQuery}
                      onSwitchToBrowser={() =>
                        sendWithTransition({ type: "SWITCH_TO_BROWSER" })
                      }
                    />
                  </Match>

                  <Match
                    when={state.matches({
                      authFlow: { enrolling: "profileCreation" }
                    })}
                  >
                    <ProfileCreationStep
                      accessToken={state.context.profileAccessToken || ""}
                      nextStep={() =>
                        sendWithTransition({ type: "CREATE_PROFILE" })
                      }
                      onValidationChange={() => {}}
                      onPendingChange={() => {}}
                      onSubmitReady={() => {}}
                    />
                  </Match>

                  <Match when={state.matches("complete")}>
                    <CompleteStep
                      hasGDLAccount={state.context.hasGDLAccount}
                      foundExistingAccount={
                        state.context.foundExistingGDLAccount
                      }
                      foundGDLAccountData={state.context.foundGDLAccountData}
                      onContinue={() => send({ type: "SKIP_GDL_ACCOUNT" })}
                      onSetupGDLAccount={() =>
                        sendWithTransition({ type: "SETUP_GDL_ACCOUNT" })
                      }
                      onLinkExistingAccount={() =>
                        sendWithTransition({ type: "LINK_GDL_ACCOUNT" })
                      }
                    />
                  </Match>
                </Switch>
              </div>
            </div>

            {/* Footer with progress and buttons */}
            <div class="box-border flex w-full flex-col items-center p-4">
              <Show when={!state.context.activeUuid}>
                <ProgressStepper
                  currentStep={state.context.currentStep}
                  totalSteps={5}
                />
              </Show>

              <div class="box-border flex w-full">
                <div
                  ref={btnRef}
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
                    onClick={() => sendWithTransition({ type: "BACK" })}
                  >
                    <div class="i-hugeicons:arrow-left-01" />
                    <Trans key="general.back" />
                  </Button>
                </div>

                <Show
                  when={
                    // @ts-expect-error - XState v5 state.matches() has type inference issues
                    state.matches("onboarding.welcome") ||
                    // @ts-expect-error - XState v5 state.matches() has type inference issues
                    state.matches("onboarding.termsAndPrivacy")
                  }
                >
                  <Button
                    fullWidth
                    variant="primary"
                    size="large"
                    disabled={
                      // @ts-expect-error - XState v5 state.matches() has type inference issues
                      state.matches("onboarding.termsAndPrivacy") &&
                      !state.context.termsAccepted
                    }
                    onClick={() => sendWithTransition({ type: "CONTINUE" })}
                  >
                    <Show
                      // @ts-expect-error - XState v5 state.matches() has type inference issues
                      when={state.matches("onboarding.welcome")}
                      fallback={
                        <>
                          <Trans key="login.agree_and_continue" />
                          <div class="i-hugeicons:arrow-right-01" />
                        </>
                      }
                    >
                      <Trans key="login.next" />
                      <div class="i-hugeicons:arrow-right-01" />
                    </Show>
                  </Button>
                </Show>

                {/* Continue to Library button for complete state (only in normal flow, not when adding from settings) */}
                <Show
                  when={
                    state.matches("complete") && !state.context.isAddingAccount
                  }
                >
                  <Button
                    size="large"
                    type="secondary"
                    fullWidth
                    onClick={() => send({ type: "SKIP_GDL_ACCOUNT" })}
                  >
                    <Trans key="login.continue_to_library" />
                    <div class="i-hugeicons:arrow-right-01" />
                  </Button>
                </Show>
              </div>
            </div>
          </div>
        </Show>

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
              <Trans key="login.welcome_to" />
            </div>
            <div ref={gdlauncherTextRef} class="opacity-0">
              <Trans key="login.gdlauncher" />
            </div>
          </div>
          <div class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 text-7xl font-bold leading-loose">
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

      {/* GDL Account Setup Modal */}
      <GDLAccountSetupModal
        isOpen={state.matches({ complete: "gdlSetup" })}
        onClose={() => send({ type: "GDL_SETUP_CANCELLED" })}
        onComplete={() => send({ type: "GDL_SETUP_COMPLETE" })}
        activeUuid={state.context.activeUuid}
      />
    </>
  )
}
