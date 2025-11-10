import type { Occasion } from "@/utils/occasions"
import type {
  EnrollmentStatus,
  EnrollmentError,
  EnrollmentErrorType,
  DeviceCode,
  AccountEntry,
  AccountType,
  AccountStatus,
  XboxError,
  FEGDLAccount
} from "@gd/core_module/bindings"

// Re-export types from bindings for convenience
export type {
  EnrollmentStatus,
  EnrollmentError,
  EnrollmentErrorType,
  DeviceCode,
  AccountEntry,
  AccountType,
  AccountStatus,
  XboxError,
  FEGDLAccount
}

// ============================================================================
// Auth Flow State - Discriminated Union for Type Safety
// ============================================================================

/**
 * Main auth flow state machine
 * Uses discriminated union for perfect TypeScript narrowing
 */
export type LoadingOperation =
  | "initializing"
  | "checking-account"
  | "checking-gdl"

export type AuthFlowState =
  // Global loading states (show spinner overlay, no sidebar)
  | {
      phase: "loading"
      operation: LoadingOperation
    }

  // Content states (show sidebar with UI)
  | {
      phase: "content"
      step: AuthStep
    }

  // Transition states (animations between steps)
  | {
      phase: "transitioning"
      from: AuthStep
      to: AuthStep
      direction: "forward" | "backward"
    }

  // Exit states (navigating away)
  | {
      phase: "exiting"
      destination: "library" | "settings"
      showSeasonal: boolean
    }

// ============================================================================
// Auth Steps - Individual Screen States
// ============================================================================

export type AuthStep =
  | { type: "welcome" }
  | { type: "terms"; variant: "initial" | "forced" }
  | { type: "auth-method" }
  | {
      type: "enrolling"
      method: "browser" | "device-code"
    }
  | { type: "profile-creation"; accessToken: string }
  | { type: "gdl-account"; gdlAccount: GDLAccountState }
  | { type: "error"; message: string; canRetry: boolean }

// EnrollmentStatus, DeviceCode, EnrollmentError types are imported from bindings

/**
 * GDL account state
 */
export type GDLAccountState =
  | { type: "none" } // No GDL account
  | { type: "found-existing"; data: FEGDLAccount } // Found existing account
  | { type: "linked" } // Already linked

// ============================================================================
// Shared Data - Mutable State Across Flow
// ============================================================================

/**
 * Shared mutable data accessible throughout the auth flow
 * Separate from step state for clarity
 */
export interface AuthSharedData {
  // User data
  activeUuid: string | null
  accounts: AccountEntry[]

  // Settings
  isFirstLaunch: boolean
  termsAndPrivacyAccepted: boolean
  reducedMotion: boolean

  // GDL account
  gdlAccountId: string | null
  hasGDLAccount: boolean
  foundGDLAccountData: FEGDLAccount | null
  pendingGDLAccountUuid: string | null

  // UI state
  termsAccepted: boolean
  currentOccasion: Occasion | null

  // Navigation context
  isAddingMicrosoftFromSettings: boolean
  isAddingGdlFromSettings: boolean
  shouldSetupGdlAfterAuth: boolean
  returnPath: string | null

  // Error state
  error: string | null
}

// AccountEntry, AccountType, AccountStatus types are imported from bindings

// ============================================================================
// Animation Types
// ============================================================================

/**
 * Animation options for timeline sequences
 */
export interface AnimationOptions {
  duration?: number
  delay?: number
  easing?: string
}

export interface RequiredDelayAnimationOptions {
  duration: number
  delay: number
  easing?: string
}

/**
 * Direction for step transitions
 */
export type TransitionDirection = "forward" | "backward"

/**
 * Ref names for animation targets
 */
export type AnimationRefName =
  | "sidebar"
  | "video"
  | "stepContainer"
  | "backgroundBlur"
  | "loadingSpinner"
  | "backButton"
  | "skipButton"
  | "welcomeToText"
  | "gdlauncherText"

/**
 * Collection of DOM refs for animations
 */
export type AnimationRefs = Partial<Record<AnimationRefName, HTMLElement>>

// ============================================================================
// Flow Controller Interface
// ============================================================================

/**
 * Main flow controller interface
 * Manages state, navigation, and data operations
 */
export interface FlowController {
  // Current state (reactive signal)
  state: () => AuthFlowState

  // Next transition direction (set before transition starts)
  nextDirection: () => TransitionDirection | null

  // Navigation
  goToStep(
    step: AuthStep,
    options?: { direction?: TransitionDirection }
  ): Promise<void>
  goBack(): Promise<void>
  exitFlow(
    destination?: "library" | "settings",
    showSeasonal?: boolean
  ): Promise<void>

  // Loading control
  showGlobalLoading(operation: LoadingOperation): void
  hideGlobalLoading(): void
  isGlobalLoading: () => boolean

  // Cleanup
  cleanup(): void

  // Data operations (may show global loading)
  acceptTerms(): Promise<void>
  startEnrollment(method: "browser" | "device-code"): Promise<void>
  cancelEnrollment(): Promise<void>
  checkUsernameAvailability(
    accessToken: string,
    username: string
  ): Promise<"available" | "taken" | "notallowed">
  createProfile(accessToken: string, username: string): Promise<void>
  checkGDLAccount(showLoading?: boolean): Promise<GDLAccountState>
  setupGDLAccount(): Promise<void>
  linkExistingGDLAccount(gdlAccountData: any): Promise<void>
  skipGDLAccount(): Promise<void>
  checkExistingAccount(): Promise<void>
  completeAuth(uuid: string): Promise<void>
  finalizeEnrollment(): Promise<void>

  // Shared data (reactive store)
  data: AuthSharedData

  // History for back navigation
  canGoBack: () => boolean
}

// ============================================================================
// Animation Controller Interface
// ============================================================================

/**
 * Animation controller interface
 * Provides declarative animation primitives
 */
export interface AnimationController {
  // Ref management
  registerRefs(refs: AnimationRefs): void
  refsReady: () => boolean

  // Animation primitives
  step: {
    slideTransition(direction: TransitionDirection): Promise<void>
  }

  sidebar: {
    slideIn(): Promise<void>
    slideOut(): Promise<void>
  }

  loading: {
    show(): Promise<void>
    hide(): Promise<void>
  }

  text: {
    fadeIn(ref: AnimationRefName, options?: AnimationOptions): Promise<void>
    fadeOut(
      ref: AnimationRefName | AnimationRefName[],
      options?: AnimationOptions
    ): Promise<void>
  }

  backgroundBlur: {
    fadeIn(options?: AnimationOptions): Promise<void>
    fadeOut(options?: AnimationOptions): Promise<void>
  }

  backButton: {
    show(): Promise<void>
    hide(): Promise<void>
  }

  skipButton: {
    show(): Promise<void>
    hide(): Promise<void>
  }

  seasonal: {
    show(occasion: Occasion): Promise<void>
    hide(): Promise<void>
  }
}

// ============================================================================
// Helper Types
// ============================================================================

/**
 * Extract step type from AuthStep union
 */
export type StepType = AuthStep["type"]

/**
 * Get step data for a specific step type
 */
export type StepData<T extends StepType> = Extract<AuthStep, { type: T }>

/**
 * Initial flow configuration
 */
export interface AuthFlowConfig {
  // User data
  activeUuid: string | null
  accounts: AccountEntry[]

  // Settings
  isFirstLaunch: boolean
  termsAndPrivacyAccepted: boolean
  gdlAccountId: string | null
  reducedMotion: boolean

  // Navigation context
  isAddingMicrosoftFromSettings: boolean
  isAddingGdlFromSettings: boolean
  shouldSetupGdlAfterAuth: boolean
  returnPath: string | null

  // UI
  currentOccasion: Occasion | null
}
