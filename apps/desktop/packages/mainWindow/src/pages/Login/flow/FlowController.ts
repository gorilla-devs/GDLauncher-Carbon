import { createSignal } from "solid-js"
import type {
  CreateMutationResult,
  CreateQueryResult
} from "@tanstack/solid-query"
import type { RSPCError } from "@rspc/client"
import type {
  FEGDLAccount,
  FESettingsUpdate,
  FECheckUsernameAvailability,
  FECreateProfile,
  UsernameAvailability
} from "@gd/core_module/bindings"
import type {
  AuthFlowState,
  AuthFlowConfig,
  AuthStep,
  AuthSharedData,
  FlowController,
  GDLAccountState,
  AccountEntry,
  LoadingOperation,
  TransitionDirection
} from "./types"
import type { rspc } from "@/utils/rspcClient"

type RSPCContext = ReturnType<typeof rspc.useContext>

/**
 * FlowController Implementation
 *
 * Manages auth flow state, navigation, and data operations
 * Uses SolidJS signals for reactive state management
 */
export class FlowControllerImpl implements FlowController {
  // Reactive state signal
  private stateSignal = createSignal<AuthFlowState>({
    phase: "loading",
    operation: "initializing"
  })

  // Signal to store the next transition direction before transition starts
  private nextDirectionSignal = createSignal<TransitionDirection | null>(null)

  // Shared mutable data store
  public data: AuthSharedData

  // History stack for back navigation
  private history: AuthStep[] = []

  // External dependencies (injected)
  private rspcContext: RSPCContext
  private settingsMutation: CreateMutationResult<
    null,
    RSPCError,
    FESettingsUpdate
  >
  private saveGdlAccountMutation: CreateMutationResult<
    null,
    RSPCError,
    string | null
  >
  private enrollBeginMutation: CreateMutationResult<null, RSPCError, undefined>
  private enrollBeginBrowserMutation: CreateMutationResult<
    null,
    RSPCError,
    boolean
  >
  private enrollCancelMutation: CreateMutationResult<null, RSPCError, undefined>
  private usernameAvailabilityMutation: CreateMutationResult<
    UsernameAvailability,
    RSPCError,
    FECheckUsernameAvailability
  >
  private createProfileMutation: CreateMutationResult<
    null,
    RSPCError,
    FECreateProfile
  >
  private enrollResumeMutation: CreateMutationResult<null, RSPCError, undefined>
  private accountsQuery: CreateQueryResult<AccountEntry[], RSPCError>
  private gdlAccountQuery: CreateQueryResult<FEGDLAccount | null, RSPCError>

  // Abort controller for cancelling pending operations
  private abortController: AbortController | null = null
  private initPromise: Promise<void> | null = null

  constructor(
    config: AuthFlowConfig,
    deps: {
      rspcContext: RSPCContext
      settingsMutation: CreateMutationResult<null, RSPCError, FESettingsUpdate>
      saveGdlAccountMutation: CreateMutationResult<
        null,
        RSPCError,
        string | null
      >
      enrollBeginMutation: CreateMutationResult<null, RSPCError, undefined>
      enrollBeginBrowserMutation: CreateMutationResult<null, RSPCError, boolean>
      enrollCancelMutation: CreateMutationResult<null, RSPCError, undefined>
      usernameAvailabilityMutation: CreateMutationResult<
        UsernameAvailability,
        RSPCError,
        FECheckUsernameAvailability
      >
      createProfileMutation: CreateMutationResult<
        null,
        RSPCError,
        FECreateProfile
      >
      enrollResumeMutation: CreateMutationResult<null, RSPCError, undefined>
      accountsQuery: CreateQueryResult<AccountEntry[], RSPCError>
      gdlAccountQuery: CreateQueryResult<FEGDLAccount | null, RSPCError>
    }
  ) {
    // Initialize shared data from config
    this.data = {
      activeUuid: config.activeUuid,
      accounts: config.accounts,
      isFirstLaunch: config.isFirstLaunch,
      termsAndPrivacyAccepted: config.termsAndPrivacyAccepted,
      reducedMotion: config.reducedMotion,
      gdlAccountId: config.gdlAccountId,
      hasGDLAccount: false,
      foundGDLAccountData: null,
      pendingGDLAccountUuid: null,
      termsAccepted: config.termsAndPrivacyAccepted,
      currentOccasion: config.currentOccasion,
      isAddingMicrosoftFromSettings: config.isAddingMicrosoftFromSettings,
      isAddingGdlFromSettings: config.isAddingGdlFromSettings,
      shouldSetupGdlAfterAuth: config.shouldSetupGdlAfterAuth,
      returnPath: config.returnPath,
      error: null
    }

    // Store dependencies
    this.rspcContext = deps.rspcContext
    this.settingsMutation = deps.settingsMutation
    this.saveGdlAccountMutation = deps.saveGdlAccountMutation
    this.enrollBeginMutation = deps.enrollBeginMutation
    this.enrollBeginBrowserMutation = deps.enrollBeginBrowserMutation
    this.enrollCancelMutation = deps.enrollCancelMutation
    this.usernameAvailabilityMutation = deps.usernameAvailabilityMutation
    this.createProfileMutation = deps.createProfileMutation
    this.enrollResumeMutation = deps.enrollResumeMutation
    this.accountsQuery = deps.accountsQuery
    this.gdlAccountQuery = deps.gdlAccountQuery

    // Create abort controller for this initialization
    this.abortController = new AbortController()

    // Start initialization (async, but don't block constructor)
    this.initPromise = this.initializeFlow().catch((error) => {
      // Don't show error if aborted (cleanup)
      if (error.name === "AbortError") {
        return
      }

      console.error("[FlowController] Initialization failed:", error)
      // Set error state
      this.setState({
        phase: "content",
        step: {
          type: "error",
          message: error.message || "Failed to initialize authentication flow",
          canRetry: true
        }
      })
    })
  }

  // ============================================================================
  // State Access
  // ============================================================================

  state = (): AuthFlowState => {
    return this.stateSignal[0]()
  }

  nextDirection = (): TransitionDirection | null => {
    return this.nextDirectionSignal[0]()
  }

  private setState = (newState: AuthFlowState): void => {
    this.stateSignal[1](newState)
  }

  // ============================================================================
  // Loading Control
  // ============================================================================

  showGlobalLoading(operation: LoadingOperation): void {
    this.setState({ phase: "loading", operation })
  }

  hideGlobalLoading(): void {
    // Don't hide if already in content/transitioning state
    const current = this.state()
    if (current.phase === "content" || current.phase === "transitioning") {
      return
    }
  }

  isGlobalLoading = (): boolean => {
    return this.state().phase === "loading"
  }

  // ============================================================================
  // Cleanup
  // ============================================================================

  cleanup(): void {
    // Abort any pending operations
    if (this.abortController) {
      this.abortController.abort()
      this.abortController = null
    }
    this.initPromise = null
  }

  // ============================================================================
  // Navigation
  // ============================================================================

  async goToStep(
    step: AuthStep,
    options?: { direction?: TransitionDirection }
  ): Promise<void> {
    const currentState = this.state()

    // Block navigation during loading
    if (currentState.phase === "loading") {
      console.warn(
        "[FlowController] Cannot navigate while loading, please wait"
      )
      return
    }

    // Can only navigate from content states
    if (currentState.phase !== "content") {
      console.warn("[FlowController] Cannot navigate from non-content state")
      return
    }

    // Add current step to history
    this.history.push(currentState.step)

    // Set direction before transition (use provided direction or default to "forward")
    const direction = options?.direction || "forward"
    this.nextDirectionSignal[1](direction)

    // Wait for SolidJS to flush DOM updates before capturing old snapshot
    await new Promise((resolve) => queueMicrotask(() => resolve(undefined)))

    // Use View Transition API for smooth transitions
    if ("startViewTransition" in document) {
      const transition = (document as any).startViewTransition(() => {
        // Only update state in callback
        this.setState({ phase: "content", step })
      })

      try {
        await transition.finished
      } catch (error) {
        // Transition was skipped, continue anyway
        console.warn("[FlowController] View transition skipped:", error)
      }

      // Clear next direction after transition
      this.nextDirectionSignal[1](null)
    } else {
      // Fallback for browsers without View Transition API support
      this.setState({ phase: "content", step })
      this.nextDirectionSignal[1](null)
    }
  }

  async goBack(): Promise<void> {
    const currentState = this.state()

    // Block navigation during loading
    if (currentState.phase === "loading") {
      console.warn("[FlowController] Cannot go back while loading, please wait")
      return
    }

    if (this.history.length === 0) {
      console.warn("[FlowController] No history to go back to")
      return
    }

    if (currentState.phase !== "content") return

    // Pop from history
    const previousStep = this.history.pop()!

    // Set direction before transition
    this.nextDirectionSignal[1]("backward")

    // Wait for SolidJS to flush DOM updates before capturing old snapshot
    await new Promise((resolve) => queueMicrotask(() => resolve(undefined)))

    // Use View Transition API for smooth transitions
    if ("startViewTransition" in document) {
      const transition = (document as any).startViewTransition(() => {
        // Only update state in callback
        this.setState({ phase: "content", step: previousStep })
      })

      try {
        await transition.finished
      } catch (error) {
        // Transition was skipped, continue anyway
        console.warn("[FlowController] View transition skipped:", error)
      }

      // Clear next direction after transition
      this.nextDirectionSignal[1](null)
    } else {
      // Fallback for browsers without View Transition API support
      this.setState({ phase: "content", step: previousStep })
      this.nextDirectionSignal[1](null)
    }
  }

  canGoBack = (): boolean => {
    return this.history.length > 0
  }

  async exitFlow(
    destination: "library" | "settings" = "library",
    showSeasonal?: boolean
  ): Promise<void> {
    this.setState({
      phase: "exiting",
      destination,
      showSeasonal:
        showSeasonal !== undefined
          ? showSeasonal
          : this.data.currentOccasion !== null
    })
  }

  // ============================================================================
  // Data Operations
  // ============================================================================

  async acceptTerms(): Promise<void> {
    await this.settingsMutation.mutateAsync({
      termsAndPrivacyAccepted: { Set: true }
    })
    this.data.termsAndPrivacyAccepted = true
  }

  async startEnrollment(method: "browser" | "device-code"): Promise<void> {
    try {
      if (method === "device-code") {
        await this.enrollBeginMutation.mutateAsync(undefined)
      } else {
        await this.enrollBeginBrowserMutation.mutateAsync(true)
      }
      // Enrollment status will be polled by EnrollingStep component
    } catch (error) {
      console.error("[FlowController] Failed to start enrollment:", error)
      throw error
    }
  }

  async cancelEnrollment(): Promise<void> {
    try {
      await this.enrollCancelMutation.mutateAsync(undefined)
    } catch (error) {
      console.error("[FlowController] Failed to cancel enrollment:", error)
    }
  }

  async checkUsernameAvailability(
    accessToken: string,
    username: string
  ): Promise<"available" | "taken" | "notallowed"> {
    try {
      const result = await this.usernameAvailabilityMutation.mutateAsync({
        accessToken,
        username
      })
      return result
    } catch (error) {
      console.error(
        "[FlowController] Failed to check username availability:",
        error
      )
      throw error
    }
  }

  async createProfile(accessToken: string, username: string): Promise<void> {
    try {
      await this.createProfileMutation.mutateAsync({
        accessToken,
        username
      })

      // Resume enrollment flow - this saves the account and sets it as active on the backend
      await this.enrollResumeMutation.mutateAsync(undefined)

      // Sync frontend state with backend after enrollment is finalized
      await Promise.all([
        this.rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getAccounts"]
        }),
        this.rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getActiveUuid"]
        })
      ])

      // Get the new active UUID from the backend
      const activeUuid = await this.rspcContext.client.query([
        "account.getActiveUuid"
      ])

      // Update local state with the newly enrolled account
      if (activeUuid) {
        this.data.activeUuid = activeUuid
      }

      // Update accounts list
      const accountsResult = await this.accountsQuery.refetch()
      this.data.accounts = accountsResult.data || []
    } catch (error) {
      console.error("[FlowController] Failed to create profile:", error)
      throw error
    }
  }

  async setupGDLAccount(): Promise<void> {
    try {
      // This would open a modal or navigate to GDL account setup
      // For now, just save an empty GDL account ID to indicate "asked but skipped"
      await this.saveGdlAccountMutation.mutateAsync("")

      // Refetch settings to update cache
      await this.rspcContext.queryClient.refetchQueries({
        queryKey: ["settings.getSettings"]
      })

      // Update local state
      this.data.gdlAccountId = ""
    } catch (error) {
      console.error("[FlowController] Failed to setup GDL account:", error)
      throw error
    }
  }

  async linkExistingGDLAccount(_gdlAccountData: any): Promise<void> {
    try {
      await this.saveGdlAccountMutation.mutateAsync(this.data.activeUuid)

      // Refetch settings to update cache
      await this.rspcContext.queryClient.refetchQueries({
        queryKey: ["settings.getSettings"]
      })

      // Update local state
      this.data.gdlAccountId = this.data.activeUuid!
    } catch (error) {
      console.error(
        "[FlowController] Failed to link existing GDL account:",
        error
      )
      throw error
    }
  }

  async skipGDLAccount(): Promise<void> {
    try {
      // Save empty string to indicate user chose to skip
      await this.saveGdlAccountMutation.mutateAsync("")

      // Refetch settings to update cache
      await this.rspcContext.queryClient.refetchQueries({
        queryKey: ["settings.getSettings"]
      })

      // Update local state
      this.data.gdlAccountId = ""
    } catch (error) {
      console.error("[FlowController] Failed to save skip GDL account:", error)
      throw error
    }
  }

  async checkGDLAccount(showLoading = false): Promise<GDLAccountState> {
    if (showLoading) {
      this.showGlobalLoading("checking-gdl")
    }

    try {
      const { activeUuid, gdlAccountId } = this.data

      if (!activeUuid) {
        return { type: "none" }
      }

      // Verify account exists
      const accounts = await this.accountsQuery.refetch()
      const accountExists = accounts.data?.some(
        (acc: any) => acc.uuid === activeUuid
      )

      if (!accountExists) {
        throw new Error(`Account ${activeUuid} not found`)
      }

      // Check cloud for GDL account
      try {
        const gdlAccount = await this.rspcContext.client.query([
          "account.peekGdlAccount",
          activeUuid
        ])

        if (gdlAccount) {
          this.data.hasGDLAccount = true
          this.data.foundGDLAccountData = gdlAccount

          // Check if it's the same as local
          if (gdlAccountId === activeUuid) {
            return { type: "linked" }
          }

          return { type: "found-existing", data: gdlAccount }
        }

        return { type: "none" }
      } catch (error) {
        console.error("[FlowController] Failed to peek GDL account:", error)
        const errorMessage =
          error instanceof Error ? error.message : "Failed to check account"
        return { type: "error", message: errorMessage }
      }
    } finally {
      // Don't hide loading here - caller is responsible
    }
  }

  async checkExistingAccount(): Promise<void> {
    try {
      // If user already skipped (empty string) AND not adding from settings, exit immediately
      if (this.data.gdlAccountId === "" && !this.data.isAddingGdlFromSettings) {
        await this.exitFlow("library")
        return
      }

      this.showGlobalLoading("checking-gdl")

      const { activeUuid, gdlAccountId } = this.data

      if (!activeUuid) {
        this.setState({
          phase: "content",
          step: { type: "gdl-account", gdlAccount: { type: "none" } }
        })
        return
      }

      // Verify account exists
      const accounts = await this.accountsQuery.refetch()
      const accountExists = accounts.data?.some(
        (acc: any) => acc.uuid === activeUuid
      )

      if (!accountExists) {
        throw new Error(`Account ${activeUuid} not found`)
      }

      // Check cloud for GDL account
      try {
        const gdlAccount = await this.rspcContext.client.query([
          "account.peekGdlAccount",
          activeUuid
        ])

        if (gdlAccount) {
          this.data.hasGDLAccount = true
          this.data.foundGDLAccountData = gdlAccount

          // Check if verified
          if (gdlAccount.isEmailVerified) {
            // Already verified - check if same as local
            if (gdlAccountId === activeUuid) {
              // Already linked - exit to library
              await this.exitFlow("library")
              return
            }
            // Found existing verified account - show sync option
            this.setState({
              phase: "content",
              step: {
                type: "gdl-account",
                gdlAccount: { type: "found-existing", data: gdlAccount }
              }
            })
          } else {
            // Unverified account - go directly to verification step
            this.setState({
              phase: "content",
              step: {
                type: "gdl-account-verification",
                email: gdlAccount.email,
                uuid: activeUuid
              }
            })
          }
          return
        }

        // No GDL account found
        this.setState({
          phase: "content",
          step: { type: "gdl-account", gdlAccount: { type: "none" } }
        })
      } catch (error) {
        console.error("[FlowController] Failed to peek GDL account:", error)
        const errorMessage =
          error instanceof Error ? error.message : "Failed to check account"
        this.setState({
          phase: "content",
          step: {
            type: "gdl-account",
            gdlAccount: { type: "error", message: errorMessage }
          }
        })
      }
    } finally {
      // Ensure loading is cleared when transitioning to content
      this.hideGlobalLoading()
    }
  }

  async completeAuth(uuid: string): Promise<void> {
    this.data.activeUuid = uuid

    // Refetch accounts
    await Promise.all([
      this.rspcContext.queryClient.refetchQueries({
        queryKey: ["account.getAccounts"]
      }),
      this.rspcContext.queryClient.refetchQueries({
        queryKey: ["account.getActiveUuid"]
      })
    ])

    // Update accounts list
    const accountsResult = await this.accountsQuery.refetch()
    this.data.accounts = accountsResult.data || []

    // If dual-flow (Microsoft + GDL), proceed to GDL setup
    if (this.data.shouldSetupGdlAfterAuth) {
      // Navigate to gdl-account step for GDL account checking
      await this.goToStep({ type: "gdl-account", gdlAccount: { type: "none" } })
      // Trigger GDL account check
      await this.checkExistingAccount()
    }
  }

  /**
   * Finalize Microsoft account enrollment
   * Saves the completed enrollment to database and sets it as active account
   */
  async finalizeEnrollment(): Promise<void> {
    try {
      // Call backend to persist the account
      await this.rspcContext.client.mutation(["account.enroll.finalize"])

      // Refetch accounts to update UI
      await Promise.all([
        this.rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getAccounts"]
        }),
        this.rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getActiveUuid"]
        })
      ])

      // Get the new active UUID from the backend and update local state
      // This is critical for checkGDLAccount() to work correctly after enrollment
      const activeUuid = await this.rspcContext.client.query([
        "account.getActiveUuid"
      ])
      if (activeUuid) {
        this.data.activeUuid = activeUuid
      }

      // Update local accounts list
      const accountsResult = await this.accountsQuery.refetch()
      this.data.accounts = accountsResult.data || []
    } catch (error) {
      console.error("[FlowController] Failed to finalize enrollment:", error)
      throw error
    }
  }

  // ============================================================================
  // Initial State Determination
  // ============================================================================

  private async initializeFlow(): Promise<void> {
    const initialStep = this.determineInitialStep()

    // If initial step requires data loading, fetch it
    if (this.requiresDataLoading(initialStep)) {
      await this.loadInitialData(initialStep)

      // Check if data loading already set the state (e.g., to "exiting" or "content")
      // If so, don't overwrite it
      const currentState = this.state()
      if (currentState.phase !== "loading") {
        return
      }
    }

    // Transition to content with initial step (only if still in loading phase)
    this.setState({ phase: "content", step: initialStep })
  }

  private determineInitialStep(): AuthStep {
    const {
      activeUuid,
      accounts,
      termsAndPrivacyAccepted,
      gdlAccountId,
      isAddingMicrosoftFromSettings,
      isAddingGdlFromSettings
    } = this.data

    // Adding Microsoft account from settings - go directly to auth
    if (isAddingMicrosoftFromSettings) {
      return { type: "auth-method" }
    }

    // Adding GDL account from settings - check for existing account
    if (isAddingGdlFromSettings) {
      // GDL account requires a Microsoft account to be logged in
      if (!activeUuid || accounts.length === 0) {
        console.warn(
          "[FlowController] Cannot add GDL account without Microsoft account"
        )
        // User needs to sign in with Microsoft first
        return { type: "auth-method" }
      }
      // Go directly to GDL account checking (will load in initializeFlow)
      return { type: "gdl-account", gdlAccount: { type: "none" } }
    }

    // Forced terms reacceptance for returning users
    if (activeUuid && accounts.length > 0 && !termsAndPrivacyAccepted) {
      return { type: "terms", variant: "forced" }
    }

    // Fast path: returning user with everything set up
    if (
      activeUuid &&
      termsAndPrivacyAccepted &&
      gdlAccountId // Truthy check: excludes null, "", and undefined
    ) {
      // User has GDL account linked - fast path
      return { type: "gdl-account", gdlAccount: { type: "none" } }
    }

    // Existing account needs GDL check
    if (activeUuid) {
      // Will load data in initializeFlow
      return { type: "gdl-account", gdlAccount: { type: "none" } }
    }

    // First-time user needs onboarding
    if (!termsAndPrivacyAccepted) {
      return { type: "welcome" }
    }

    // Default: go to auth
    return { type: "auth-method" }
  }

  private requiresDataLoading(step: AuthStep): boolean {
    // Steps that need data before showing
    if (step.type === "gdl-account" && this.data.activeUuid) {
      return true
    }

    return false
  }

  private async loadInitialData(step: AuthStep): Promise<void> {
    if (step.type === "gdl-account" && this.data.activeUuid) {
      // Add 10-second timeout
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(
          () => reject(new Error("Request timed out after 10 seconds")),
          10000
        )
      })

      // Race between data loading and timeout
      await Promise.race([this.checkExistingAccount(), timeoutPromise])
    }
  }

  // ============================================================================
  // Private Helpers
  // ============================================================================

  private waitForTransition(): Promise<void> {
    // Transition duration (should match animation)
    return new Promise((resolve) => setTimeout(resolve, 400))
  }
}

/**
 * Create a flow controller instance
 */
export function createFlowController(
  config: AuthFlowConfig,
  deps: {
    rspcContext: RSPCContext
    settingsMutation: CreateMutationResult<null, RSPCError, FESettingsUpdate>
    saveGdlAccountMutation: CreateMutationResult<null, RSPCError, string | null>
    enrollBeginMutation: CreateMutationResult<null, RSPCError, undefined>
    enrollBeginBrowserMutation: CreateMutationResult<null, RSPCError, boolean>
    enrollCancelMutation: CreateMutationResult<null, RSPCError, undefined>
    usernameAvailabilityMutation: CreateMutationResult<
      UsernameAvailability,
      RSPCError,
      FECheckUsernameAvailability
    >
    createProfileMutation: CreateMutationResult<
      null,
      RSPCError,
      FECreateProfile
    >
    enrollResumeMutation: CreateMutationResult<null, RSPCError, undefined>
    accountsQuery: CreateQueryResult<AccountEntry[], RSPCError>
    gdlAccountQuery: CreateQueryResult<FEGDLAccount | null, RSPCError>
  }
): FlowController {
  return new FlowControllerImpl(config, deps)
}
