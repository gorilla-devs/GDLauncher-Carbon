import { Button, Spinner, AnimatedIcon } from "@gd/ui"
import {
  createSignal,
  Switch,
  Match,
  createEffect,
  onMount,
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

// Import hooks
import {
  useAuthFlow,
  useEnrollmentStatus,
  useAuthTransitions,
  useAuthAnimations
} from "../hooks"

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
import type { Occasion } from "@/utils/occasions"

// Import types
import type { DeviceCodeObjectType } from "../index"

// Import styles
import "../styles/viewTransitions.css"

/**
 * Main Login Container Component
 *
 * Orchestrates the entire authentication flow using extracted hooks and modular components.
 * Manages a simplified 4-step flow: Welcome → Auth Method → Auth Flow → Complete
 */

enum LoginStep {
  Welcome = 1,
  TermsAndPrivacy = 2,
  AuthMethod = 3,
  AuthFlow = 4,
  Complete = 5
}

enum AuthFlowType {
  None,
  Browser,
  DeviceCode,
  ProfileCreation
}

export function LoginContainer() {
  const globalStore = useGlobalStore()
  const navigator = useGDNavigate()
  const rspcContext = rspc.useContext()

  // Detect if coming from Settings to add account
  const [searchParams] = useSearchParams()
  const isAddingAccount = searchParams.addMicrosoftAccount === "true"
  const returnPath = searchParams.returnTo || null

  // Detect occasion immediately (before any rendering)
  const initialOccasion = getCurrentOccasion()

  // Determine video source at initialization (static, never changes)
  const videoSource = initialOccasion
    ? initialOccasion.assets.authVideo
    : BackgroundVideo

  // Step management
  const { step, transitionDirection, transitionToStep, prevStep, canGoBack } =
    useAuthTransitions<LoginStep>({
      initialStep: LoginStep.Welcome,
      minStep: LoginStep.Welcome,
      maxStep: LoginStep.Complete,
      shouldTransition: () =>
        !globalStore.settings.data?.reducedMotion &&
        typeof document !== "undefined" &&
        "startViewTransition" in document
    })

  // Auth flow state
  const [authFlowType, setAuthFlowType] = createSignal<AuthFlowType>(
    AuthFlowType.None
  )
  const [deviceCodeObject, setDeviceCodeObject] =
    createSignal<DeviceCodeObjectType | null>(null)
  const [isBackButtonVisible, setIsBackButtonVisible] = createSignal(false)
  const [gdlModalOpen, setGdlModalOpen] = createSignal(false)
  const [termsAccepted, setTermsAccepted] = createSignal(false)
  const [hasGDLAccount, setHasGDLAccount] = createSignal(false)
  const [foundExistingGDLAccount, setFoundExistingGDLAccount] = createSignal(false)
  const [foundGDLAccountData, setFoundGDLAccountData] = createSignal<any | null>(null)
  const [pendingGDLAccountUuid, setPendingGDLAccountUuid] = createSignal<string | null>(null)

  // Profile creation state
  const [profileCreationValid, setProfileCreationValid] = createSignal(false)
  const [profileCreationPending, setProfileCreationPending] = createSignal(false)
  const [profileCreationSubmit, setProfileCreationSubmit] = createSignal<(() => void) | null>(null)

  // Seasonal splash state - Initialize with detected occasion
  const [currentOccasion, setCurrentOccasion] = createSignal<Occasion | null>(initialOccasion)
  const [seasonalMessageVisible, setSeasonalMessageVisible] = createSignal(false)
  const [seasonalButtonVisible, setSeasonalButtonVisible] = createSignal(false)
  let seasonalAutoAdvanceTimeout: number | undefined

  // Mutation to save GDL account to local settings
  const saveGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.saveGdlAccount"]
  }))

  // Auth flow hook
  const authFlow = useAuthFlow({
    onPollingCode: (deviceCode) => {
      // Only transition if we're currently on AuthMethod or already in AuthFlow
      // This prevents race conditions when user has manually navigated away
      if (step() === LoginStep.AuthMethod || step() === LoginStep.AuthFlow) {
        setDeviceCodeObject({
          userCode: deviceCode.userCode,
          link: deviceCode.link,
          expiresAt: deviceCode.expiresAt
        })
        setAuthFlowType(AuthFlowType.DeviceCode)
        transitionToStep(LoginStep.AuthFlow)
      }
      authFlow.setLoadingButton(false)
    },
    onWaitingForBrowser: (info) => {
      // Only transition if we're currently on AuthMethod or already in AuthFlow
      if (step() === LoginStep.AuthMethod || step() === LoginStep.AuthFlow) {
        setAuthFlowType(AuthFlowType.Browser)
        transitionToStep(LoginStep.AuthFlow)
      }
      authFlow.setLoadingButton(false)
    },
    onNeedsProfileCreation: (accessToken) => {
      // Only transition if we're currently in AuthFlow (this comes after initial auth)
      if (step() === LoginStep.AuthFlow) {
        authFlow.setProfileAccessToken(accessToken)
        setAuthFlowType(AuthFlowType.ProfileCreation)
        transitionToStep(LoginStep.AuthFlow)
      }
      authFlow.setLoadingButton(false)
    },
    onError: (error) => {
      console.error("Auth error:", error)
      transitionToStep(LoginStep.AuthMethod)
      authFlow.setLoadingButton(false)
    },
    onComplete: async () => {
      // Save the Microsoft account
      await authFlow.finalizeEnrollment()

      // Refetch accounts and active UUID to ensure we have fresh data
      await Promise.all([
        rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getAccounts"]
        }),
        rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getActiveUuid"]
        })
      ])

      // If adding account from Settings, return to Settings page
      if (isAddingAccount && returnPath) {
        authFlow.setLoadingButton(false)
        navigator.navigate(returnPath)
        return
      }

      // Get the freshly saved active UUID
      const activeUuid = await rspcContext.client.query(["account.getActiveUuid"])

      if (activeUuid) {
        // Verify the account exists in the accounts table
        const accounts = await rspcContext.client.query(["account.getAccounts"])
        const accountExists = accounts.some(acc => acc.uuid === activeUuid)

        if (!accountExists) {
          console.error(`[GDL Account Link] ERROR: Account with UUID ${activeUuid} not found in accounts table`)
          console.error('[GDL Account Link] Available account UUIDs:', accounts.map(a => a.uuid))
          setHasGDLAccount(false)
          transitionToStep(LoginStep.Complete)
          authFlow.setLoadingButton(false)
          return
        }

        try {
          const gdlAccount = await rspcContext.client.query([
            "account.peekGdlAccount",
            activeUuid
          ])

          if (gdlAccount) {
            // Found existing account - let user choose whether to link
            setHasGDLAccount(false)
            setFoundExistingGDLAccount(true)
            setFoundGDLAccountData(gdlAccount)
            setPendingGDLAccountUuid(activeUuid)
            transitionToStep(LoginStep.Complete)
          } else {
            // No GDL account, show Complete step with account creation option
            setHasGDLAccount(false)
            setFoundExistingGDLAccount(false)
            transitionToStep(LoginStep.Complete)
          }
        } catch (error) {
          console.error("Failed to peek GDL account:", error)
          // On error, show Complete step to be safe
          setHasGDLAccount(false)
          setFoundExistingGDLAccount(false)
          transitionToStep(LoginStep.Complete)
        }
      } else {
        setHasGDLAccount(false)
        transitionToStep(LoginStep.Complete)
      }

      authFlow.setLoadingButton(false)
    }
  })

  // Enrollment status hook
  const enrollmentStatus = useEnrollmentStatus(authFlow.enrollmentStatus)

  // Animation hook
  const animations = useAuthAnimations({
    isFirstLaunch: !!globalStore.settings.data?.isFirstLaunch,
    onAnimationComplete: () => {
      navigator.navigate("/library")
    }
  })

  // Refs
  let sidebarRef: HTMLDivElement | undefined
  let backgroundBlurRef: HTMLDivElement | undefined
  let loadingSpinnerRef: HTMLDivElement | undefined
  let videoRef: HTMLVideoElement | undefined
  let btnRef: HTMLDivElement | undefined

  // Handle sidebar slide-in animation on mount
  const handleSidebarAnimation = async () => {
    await new Promise((resolve) => setTimeout(resolve, 300))

    sidebarRef?.animate(
      [{ transform: "translateX(-100%)" }, { transform: "translateX(0)" }],
      {
        duration: 300,
        delay: 200,
        easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
        fill: "forwards"
      }
    )

    videoRef?.animate(
      [{ transform: "translateX(0)" }, { transform: "translateX(15%)" }],
      {
        duration: 300,
        delay: 200,
        easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
        fill: "forwards"
      }
    )

    loadingSpinnerRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 300,
      delay: 0,
      easing: "linear",
      fill: "forwards"
    })

    backgroundBlurRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 500,
      delay: 0,
      easing: "linear",
      fill: "forwards"
    })
  }

  // Handle back button animation
  const handleBackButtonAnimationForward = () => {
    if (btnRef && !isBackButtonVisible()) {
      setIsBackButtonVisible(true)
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
    }
  }

  const handleBackButtonAnimationBackward = () => {
    // Hide button when transitioning TO Welcome step (i.e., when currently on TermsAndPrivacy/step 2)
    if (btnRef && isBackButtonVisible() && step() === LoginStep.TermsAndPrivacy) {
      setIsBackButtonVisible(false)
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

  // Handle linking existing GDL account
  const handleLinkExistingAccount = async () => {
    const uuidToLink = pendingGDLAccountUuid()
    if (uuidToLink) {
      console.log('[GDL Account] User chose to link existing account:', uuidToLink)
      await saveGdlAccountMutation.mutateAsync(uuidToLink)
      setHasGDLAccount(true)
      setFoundExistingGDLAccount(false)

      // Always play welcome animation and navigate to library
      // (Don't show seasonal splash during onboarding flow)
      animations.playWelcomeAnimation()
    }
  }

  // Handle protocol URL callback from Electron
  createEffect(() => {
    const handleProtocolUrl = async (_event: any, url: string) => {
      // TODO: Call RSPC mutation to handle protocol callback
      console.log("Protocol URL received:", url)
      // await rspcContext.client.mutation(["account.enroll.protocolCallback", url])
    }

    // Listen for protocol-url event from Electron main process via IpcRenderer
    window.ipcRenderer?.on?.("protocol-url", handleProtocolUrl)

    return () => {
      window.ipcRenderer?.off?.("protocol-url", handleProtocolUrl)
    }
  })

  // Initialize on mount
  onMount(async () => {
    // Force refetch settings to ensure we have latest data
    // (Important when coming from Settings after removing GDL account)
    await rspcContext.queryClient.refetchQueries({
      queryKey: ["settings.getSettings"]
    })

    // Check if terms are accepted
    if (!globalStore.settings.data?.termsAndPrivacyAccepted) {
      // If adding account from Settings, skip welcome and go to auth
      if (isAddingAccount) {
        transitionToStep(LoginStep.AuthMethod)
        setIsBackButtonVisible(true)
        requestAnimationFrame(() => {
          handleSidebarAnimation()
        })
        return
      }

      transitionToStep(LoginStep.Welcome)
      setIsBackButtonVisible(false)
      requestAnimationFrame(() => {
        handleSidebarAnimation()
      })
      return
    }

    // If adding account from Settings, skip to AuthMethod
    if (isAddingAccount) {
      transitionToStep(LoginStep.AuthMethod)
      setIsBackButtonVisible(true)
      requestAnimationFrame(() => {
        handleSidebarAnimation()
      })
      return
    }

    // Check if already has Microsoft account
    const activeUuid = globalStore.currentlySelectedAccountUuid.data
    if (activeUuid) {
      // Verify the account exists in the accounts table
      const accounts = await rspcContext.client.query(["account.getAccounts"])
      const accountExists = accounts.some(acc => acc.uuid === activeUuid)
      if (!accountExists) {
        console.error(`[GDL Account Link onMount] ERROR: Active UUID ${activeUuid} not found in accounts table`)
        console.error('[GDL Account Link onMount] Available account UUIDs:', accounts.map(a => a.uuid))
        // Account doesn't exist, go to auth flow
        setHasGDLAccount(false)
        transitionToStep(LoginStep.AuthMethod)
        setIsBackButtonVisible(true)
        requestAnimationFrame(() => {
          handleSidebarAnimation()
        })
        return
      }

      // Query backend to check if GDL account exists using peekGdlAccount
      // This queries the cloud/backend, unlike getGdlAccount which reads from settings
      try {
        const gdlAccount = await rspcContext.client.query([
          "account.peekGdlAccount",
          activeUuid
        ])

        if (gdlAccount) {
          // Check if already linked in settings
          const localGdlId = globalStore.settings.data?.gdlAccountId
          if (localGdlId === activeUuid) {
            // Already linked to this account
            setHasGDLAccount(true)

            // Check if occasion was detected on initialization
            if (initialOccasion) {
              // Show seasonal splash instead of going directly to library
              // Hide loading spinner and blur overlay immediately
              loadingSpinnerRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
                duration: 300,
                easing: "linear",
                fill: "forwards"
              })

              backgroundBlurRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
                duration: 500,
                easing: "linear",
                fill: "forwards"
              })

              // Start timing sequence for text/button appearance
              setTimeout(() => setSeasonalMessageVisible(true), 500)
              setTimeout(() => setSeasonalButtonVisible(true), 1000)
              seasonalAutoAdvanceTimeout = setTimeout(() => {
                handleSeasonalContinue()
              }, initialOccasion.duration) as unknown as number

              return // Skip sidebar animation
            } else {
              // No occasion - go straight to library
              animations.playWelcomeAnimation()
            }
          } else {
            // Found but not linked - show choice
            setHasGDLAccount(false)
            setFoundExistingGDLAccount(true)
            setFoundGDLAccountData(gdlAccount)
            setPendingGDLAccountUuid(activeUuid)
            transitionToStep(LoginStep.Complete)
            setIsBackButtonVisible(true)
          }
        } else {
          // No account found - show create option
          setHasGDLAccount(false)
          setFoundExistingGDLAccount(false)
          transitionToStep(LoginStep.Complete)
          setIsBackButtonVisible(true)
        }
      } catch (error) {
        console.error("Failed to peek GDL account on mount:", error)
        // On error, show CompleteStep to be safe
        setHasGDLAccount(false)
        transitionToStep(LoginStep.Complete)
        setIsBackButtonVisible(true)
      }
    } else {
      // No Microsoft account, start auth flow
      setHasGDLAccount(false)
      transitionToStep(LoginStep.AuthMethod)
      setIsBackButtonVisible(true)
    }

    // Only skip sidebar animation if user is fully logged in AND there's an occasion
    if (!(currentOccasion() && hasGDLAccount())) {
      requestAnimationFrame(() => {
        handleSidebarAnimation()
      })
    } else {
      // Hide loading spinner and blur when showing seasonal content
      // (but skip sidebar animation since sidebar is hidden)
      requestAnimationFrame(() => {
        loadingSpinnerRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
          duration: 300,
          easing: "linear",
          fill: "forwards"
        })

        backgroundBlurRef?.animate([{ opacity: 1 }, { opacity: 0 }], {
          duration: 500,
          easing: "linear",
          fill: "forwards"
        })
      })
    }
  })

  onCleanup(() => {
    // Clear seasonal timeout if component unmounts
    if (seasonalAutoAdvanceTimeout !== undefined) {
      clearTimeout(seasonalAutoAdvanceTimeout)
    }
  })

  // Get step title
  const getStepTitle = () => {
    switch (step()) {
      case LoginStep.Welcome:
        return <Trans key="login.titles.welcome_to_gdlauncher" />
      case LoginStep.TermsAndPrivacy:
        return <Trans key="login.titles.terms_and_privacy" />
      case LoginStep.AuthMethod:
        return <Trans key="login.titles.sign_in_with_microsoft" />
      case LoginStep.AuthFlow:
        if (authFlowType() === AuthFlowType.Browser) {
          return <Trans key="login.titles.browser_authentication" />
        } else if (authFlowType() === AuthFlowType.DeviceCode) {
          return <Trans key="login.titles.microsoft_code_step" />
        } else if (authFlowType() === AuthFlowType.ProfileCreation) {
          return <Trans key="login.titles.create_profile" />
        }
        return <Trans key="login.titles.authentication" />
      case LoginStep.Complete:
        return <Trans key="login.titles.all_set" />
      default:
        return ""
    }
  }

  // Handle back button click
  const handleBackClick = async () => {
    // If adding account from Settings and on AuthMethod, return to Settings
    if (isAddingAccount && returnPath && step() === LoginStep.AuthMethod) {
      navigator.navigate(returnPath)
      return
    }

    if (step() === LoginStep.Complete) {
      // Skip to library
      navigator.navigate("/library")
    } else if (step() === LoginStep.AuthFlow && authFlowType() !== AuthFlowType.None) {
      // Cancel enrollment and transition back
      // Silently ignore errors if no enrollment is active
      try {
        await authFlow.cancelEnrollment()
      } catch (err) {
        console.log('[LoginContainer] Cancel enrollment failed (likely no active enrollment):', err)
      }
      setAuthFlowType(AuthFlowType.None)
      transitionToStep(LoginStep.AuthMethod)
    } else {
      handleBackButtonAnimationBackward()
      prevStep()
    }

    authFlow.setLoadingButton(false)
  }

  // Handle continue button click
  const handleContinueClick = async () => {
    handleBackButtonAnimationForward()

    if (step() === LoginStep.Welcome) {
      // Simple welcome step - just move to terms
      transitionToStep(LoginStep.TermsAndPrivacy)
    } else if (step() === LoginStep.TermsAndPrivacy) {
      // Accept terms and move to auth
      authFlow.setLoadingButton(true)
      try {
        // TODO: Save terms acceptance to backend
        authFlow.setLoadingButton(false)
        transitionToStep(LoginStep.AuthMethod)
      } catch (err) {
        console.error("Error accepting terms:", err)
        authFlow.setLoadingButton(false)
      }
    } else if (
      step() === LoginStep.AuthFlow &&
      authFlowType() === AuthFlowType.ProfileCreation
    ) {
      // Trigger profile creation
      const submitFn = profileCreationSubmit()
      if (submitFn) {
        submitFn()
      }
    } else if (step() === LoginStep.Complete && hasGDLAccount()) {
      // Let's Go - trigger welcome animation and go to library
      animations.playWelcomeAnimation()
    }
  }

  // Check if continue button should be shown
  const shouldShowContinue = () => {
    // Show continue button for ProfileCreation step
    if (
      step() === LoginStep.AuthFlow &&
      authFlowType() === AuthFlowType.ProfileCreation
    ) {
      return true
    }

    // Show "Let's Go" button on Complete step when hasGDLAccount is true
    if (step() === LoginStep.Complete && hasGDLAccount()) {
      return true
    }

    return (
      step() !== LoginStep.AuthMethod &&
      step() !== LoginStep.AuthFlow &&
      step() !== LoginStep.Complete
    )
  }

  // Check if continue button should be disabled
  const isContinueDisabled = () => {
    if (step() === LoginStep.Welcome) {
      // Welcome step has no form, never disabled
      return false
    }
    if (step() === LoginStep.TermsAndPrivacy) {
      // Disabled if terms not accepted
      return !termsAccepted() || authFlow.loadingButton()
    }
    // Check ProfileCreation validation
    if (
      step() === LoginStep.AuthFlow &&
      authFlowType() === AuthFlowType.ProfileCreation
    ) {
      return !profileCreationValid() || profileCreationPending()
    }
    return authFlow.loadingButton()
  }

  // Get back button text based on current step
  const getBackButtonText = () => {
    switch (step()) {
      case LoginStep.AuthFlow:
        return <Trans key="general.cancel" />
      case LoginStep.Complete:
        return <Trans key="login.skip_to_library" />
      default:
        return <Trans key="general.back" />
    }
  }

  // Get back button icon
  const getBackButtonIcon = () => {
    if (step() === LoginStep.Complete) {
      return <AnimatedIcon icon="i-hugeicons:arrow-right-01" />
    }
    return <AnimatedIcon icon="i-hugeicons:arrow-left-01" />
  }

  // Handle seasonal splash continue action
  const handleSeasonalContinue = () => {
    // Clear the auto-advance timeout
    if (seasonalAutoAdvanceTimeout !== undefined) {
      clearTimeout(seasonalAutoAdvanceTimeout)
    }
    // Clear occasion and navigate to library
    setCurrentOccasion(null)
    navigator.navigate("/library")
  }

  return (
    <>
      {/* Seasonal Content Overlay - Only show when fully logged in */}
      <Show when={hasGDLAccount() && currentOccasion()}>
        <>
          {/* Dark Overlay */}
          <div class="absolute inset-0 bg-black/30 z-40" />

          {/* Seasonal Content */}
          <div class="absolute inset-0 z-50 flex flex-col items-center justify-center">
            {/* Seasonal Message */}
            <div
              class="mb-8 text-center transition-all duration-1000 ease-out"
              classList={{
                "opacity-0 translate-y-5": !seasonalMessageVisible(),
                "opacity-100 translate-y-0": seasonalMessageVisible()
              }}
            >
              <h1
                class="text-7xl font-bold leading-tight"
                style={{
                  color: currentOccasion()!.colors.primary,
                  "text-shadow": `0 0 40px ${currentOccasion()!.colors.accent}, 0 4px 20px rgba(0, 0, 0, 0.5)`
                }}
              >
                {currentOccasion()!.message}
              </h1>
            </div>

            {/* Continue Button */}
            <Show when={seasonalButtonVisible()}>
              <div
                class="transition-all duration-500 ease-out"
                classList={{
                  "opacity-0 translate-y-2.5": !seasonalButtonVisible(),
                  "opacity-100 translate-y-0": seasonalButtonVisible()
                }}
              >
                <Button
                  size="large"
                  variant="primary"
                  onClick={handleSeasonalContinue}
                  style={{
                    "background-color": currentOccasion()!.colors.primary,
                    "border-color": currentOccasion()!.colors.accent
                  }}
                >
                  Continue to Library
                  <AnimatedIcon icon="i-hugeicons:arrow-right-01" class="ml-2" />
                </Button>
              </div>
            </Show>
          </div>
        </>
      </Show>

      <div class="flex h-screen w-full" id="main-login-page">
        {/* Sidebar - Hidden only when user is fully logged in AND there's an occasion */}
        <Show when={!(currentOccasion() && hasGDLAccount())}>
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
                "will-change": "transform, opacity",
                "view-transition-name": "step-content",
                // @ts-ignore - view-transition-class is a valid CSS property but not in types
                "view-transition-class": transitionDirection()
              }}
            >
              <Switch>
                <Match when={step() === LoginStep.Welcome}>
                  <WelcomeStep
                    hasActiveAccount={
                      !!globalStore.currentlySelectedAccountUuid.data
                    }
                  />
                </Match>

                <Match when={step() === LoginStep.TermsAndPrivacy}>
                  <TermsAndPrivacyStep
                    initialAccepted={termsAccepted()}
                    onAcceptanceChange={setTermsAccepted}
                  />
                </Match>

                <Match when={step() === LoginStep.AuthMethod}>
                  <AuthMethodStep
                    onBrowserAuth={() => {
                      setAuthFlowType(AuthFlowType.Browser)
                      authFlow.beginBrowserEnrollment()
                    }}
                    onDeviceCodeAuth={() => {
                      console.log('[LoginContainer] onDeviceCodeAuth called')
                      setAuthFlowType(AuthFlowType.DeviceCode)
                      authFlow.beginEnrollment()
                    }}
                    loading={authFlow.loadingButton()}
                  />
                </Match>

                <Match when={step() === LoginStep.AuthFlow}>
                  <Switch>
                    <Match when={authFlowType() === AuthFlowType.Browser}>
                      <BrowserAuthStep
                        authUrl={enrollmentStatus.browserAuthInfo()?.auth_url}
                        redirectUri={
                          enrollmentStatus.browserAuthInfo()?.redirect_uri
                        }
                        expiresAt={
                          enrollmentStatus.browserAuthInfo()?.expires_at
                        }
                        currentStage={enrollmentStatus.currentStep()}
                        isEnrolling={enrollmentStatus.isEnrolling()}
                        onSwitchToDeviceCode={() => {
                          setAuthFlowType(AuthFlowType.DeviceCode)
                          authFlow.beginEnrollment()
                        }}
                        onCancel={async () => {
                          // Cancel enrollment and transition back
                          try {
                            await authFlow.cancelEnrollment()
                          } catch (err) {
                            console.log('[BrowserAuthStep] Cancel enrollment failed:', err)
                          }
                          transitionToStep(LoginStep.AuthMethod)
                          setAuthFlowType(AuthFlowType.None)
                        }}
                        onRetry={() => {
                          authFlow.beginBrowserEnrollment()
                        }}
                      />
                    </Match>

                    <Match when={authFlowType() === AuthFlowType.DeviceCode}>
                      <DeviceCodeStepEnhanced
                        deviceCodeObject={deviceCodeObject()}
                        setDeviceCodeObject={setDeviceCodeObject}
                        nextStep={() => transitionToStep(LoginStep.Complete)}
                        prevStep={() => transitionToStep(LoginStep.AuthMethod)}
                        enrollmentStatus={authFlow.enrollmentStatus}
                        onSwitchToBrowser={() => {
                          setAuthFlowType(AuthFlowType.Browser)
                          authFlow.beginBrowserEnrollment()
                        }}
                      />
                    </Match>

                    <Match
                      when={authFlowType() === AuthFlowType.ProfileCreation}
                    >
                      <ProfileCreationStep
                        accessToken={authFlow.profileAccessToken()}
                        nextStep={() => transitionToStep(LoginStep.Complete)}
                        onValidationChange={setProfileCreationValid}
                        onPendingChange={setProfileCreationPending}
                        onSubmitReady={(fn) => setProfileCreationSubmit(() => fn)}
                      />
                    </Match>
                  </Switch>
                </Match>

                <Match when={step() === LoginStep.Complete}>
                  <CompleteStep
                    hasGDLAccount={hasGDLAccount()}
                    foundExistingAccount={foundExistingGDLAccount()}
                    foundGDLAccountData={foundGDLAccountData()}
                    onContinue={async () => {
                      // User chose to skip - save empty string to record decision
                      console.log("[GDL Account] User chose to skip linking")
                      await saveGdlAccountMutation.mutateAsync("")
                      animations.playWelcomeAnimation()
                    }}
                    onSetupGDLAccount={() => {
                      setGdlModalOpen(true)
                    }}
                    onLinkExistingAccount={handleLinkExistingAccount}
                  />
                </Match>
              </Switch>
            </div>
          </div>

          {/* Footer with progress and buttons */}
          <div class="box-border flex w-full flex-col items-center p-4">
            <Show when={!globalStore.currentlySelectedAccountUuid.data}>
              <ProgressStepper
                currentStep={step()}
                totalSteps={5}
              />
            </Show>

            <div class="box-border flex w-full">
              <Show when={!(step() === LoginStep.Complete && hasGDLAccount())}>
                <div
                  ref={btnRef}
                  class="overflow-hidden"
                  style={{
                    width: !isBackButtonVisible() ? "0" : "60%",
                    margin: !isBackButtonVisible() ? "0" : "0 1rem 0 0"
                  }}
                >
                  <Button
                    size="large"
                    type="secondary"
                    fullWidth
                    onClick={handleBackClick}
                  >
                    {getBackButtonIcon()}
                    {getBackButtonText()}
                  </Button>
                </div>
              </Show>

              <Show when={shouldShowContinue()}>
                <Button
                  fullWidth
                  variant="primary"
                  size="large"
                  disabled={isContinueDisabled()}
                  loading={
                    authFlow.loadingButton() ||
                    (step() === LoginStep.AuthFlow &&
                      authFlowType() === AuthFlowType.ProfileCreation &&
                      profileCreationPending())
                  }
                  onClick={handleContinueClick}
                >
                  <Show
                    when={step() === LoginStep.Welcome}
                    fallback={
                      <Show
                        when={step() === LoginStep.TermsAndPrivacy}
                        fallback={
                          <Show
                            when={
                              step() === LoginStep.AuthFlow &&
                              authFlowType() === AuthFlowType.ProfileCreation
                            }
                            fallback={
                              <Show
                                when={step() === LoginStep.Complete && hasGDLAccount()}
                                fallback={
                                  <>
                                    <Trans key="login.next" />
                                    <AnimatedIcon icon="i-hugeicons:arrow-right-01" />
                                  </>
                                }
                              >
                                <Trans key="login.lets_go" />
                                <AnimatedIcon icon="i-hugeicons:arrow-right-01" class="ml-2" />
                              </Show>
                            }
                          >
                            <Trans key="profile_creation.create" />
                            <AnimatedIcon icon="i-hugeicons:arrow-right-01" class="ml-2" />
                          </Show>
                        }
                      >
                        <Trans key="login.agree_and_continue" />
                        <AnimatedIcon icon="i-hugeicons:arrow-right-01" />
                      </Show>
                    }
                  >
                    <Trans key="login.next" />
                    <AnimatedIcon icon="i-hugeicons:arrow-right-01" />
                  </Show>
                </Button>
              </Show>
            </div>
          </div>
          </div>
        </Show>

        {/* Background video - Always visible, changes source based on occasion */}
        <div class="w-full flex-1">
          <div
            ref={backgroundBlurRef}
            class="z-1 absolute left-0 top-0 h-screen w-full bg-black/20 p-0"
            style={{
              "backdrop-filter": "blur(6px)"
            }}
          />
          <div class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 text-7xl font-bold leading-loose">
            <div ref={(el) => animations.setWelcomeToTextRef(el)} class="opacity-0">
              <Trans key="login.welcome_to" />
            </div>
            <div ref={(el) => animations.setGdlauncherTextRef(el)} class="opacity-0">
              <Trans key="login.gdlauncher" />
            </div>
          </div>
          <div class="z-1 absolute left-0 top-0 flex h-screen w-full flex-col items-center justify-center p-0 text-7xl font-bold leading-loose">
            <div ref={loadingSpinnerRef}>
              <Spinner class="h-10 w-10" />
            </div>
          </div>
          <video
            ref={(el) => {
              videoRef = el
              animations.setVideoRef(el)
            }}
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
        isOpen={gdlModalOpen()}
        onClose={() => setGdlModalOpen(false)}
        onComplete={() => {
          setGdlModalOpen(false)
          animations.playWelcomeAnimation()
        }}
        activeUuid={globalStore.currentlySelectedAccountUuid.data}
      />
    </>
  )
}
