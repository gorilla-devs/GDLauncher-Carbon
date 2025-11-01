import type { DeviceCodeObjectType } from "../index"
import type { Occasion } from "@/utils/occasions"

export enum LoginStep {
  Welcome = 1,
  TermsAndPrivacy = 2,
  AuthMethod = 3,
  AuthFlow = 4,
  Complete = 5
}

export enum AuthFlowType {
  None,
  Browser,
  DeviceCode,
  ProfileCreation
}

export interface GDLAccountData {
  profileIconUrl: string
  nickname: string
  email: string
}

export interface LoginMachineContext {
  // User data
  activeUuid: string | null
  accounts: any[]

  // GDL Account state
  hasGDLAccount: boolean
  foundExistingGDLAccount: boolean
  foundGDLAccountData: GDLAccountData | null
  pendingGDLAccountUuid: string | null
  gdlAccountId: string | null

  // Auth flow state
  authFlowType: AuthFlowType
  deviceCodeObject: DeviceCodeObjectType | null
  profileAccessToken: string | null
  enrollmentStatus: null | {
    waitingForBrowser?: {
      auth_url: string
      redirect_uri: string
      expires_at: number
    }
  }

  // UI state
  currentStep: LoginStep
  transitionDirection: "forward" | "backward"
  loadingButton: boolean
  termsAccepted: boolean

  // Profile creation state
  profileCreationValid: boolean
  profileCreationPending: boolean
  profileCreationSubmit: (() => void) | null

  // Seasonal state
  currentOccasion: Occasion | null
  seasonalMessageVisible: boolean
  seasonalButtonVisible: boolean

  // Navigation params
  isAddingAccount: boolean // Computed as isAddingMicrosoftAccount || isAddingGdlAccount
  isAddingMicrosoftAccount: boolean
  isAddingGdlAccount: boolean
  returnPath: string | null

  // Settings
  isFirstLaunch: boolean
  termsAndPrivacyAccepted: boolean
  reducedMotion: boolean

  // DOM refs (for animations)
  refs: {
    sidebar?: HTMLDivElement
    video?: HTMLVideoElement
    backgroundBlur?: HTMLDivElement
    loadingSpinner?: HTMLDivElement
    backButton?: HTMLDivElement
    welcomeToText?: HTMLDivElement
    gdlauncherText?: HTMLDivElement
  }

  // Error state
  error: string | null
}

export type LoginMachineEvents =
  | { type: "CONTINUE" }
  | { type: "BACK" }
  | { type: "ACCEPT_TERMS" }
  | { type: "SELECT_BROWSER_AUTH" }
  | { type: "SELECT_DEVICE_CODE" }
  | { type: "SWITCH_TO_BROWSER" }
  | { type: "SWITCH_TO_DEVICE_CODE" }
  | { type: "CANCEL_AUTH" }
  | { type: "RETRY_AUTH" }
  | { type: "CREATE_PROFILE" }
  | { type: "SETUP_GDL_ACCOUNT" }
  | { type: "LINK_GDL_ACCOUNT" }
  | { type: "SKIP_GDL_ACCOUNT" }
  | { type: "CONTINUE_SEASONAL" }
  | { type: "POLLING_CODE_RECEIVED"; data: DeviceCodeObjectType }
  | {
      type: "BROWSER_AUTH_READY"
      data: { authUrl: string; redirectUri: string; expiresAt: number }
    }
  | { type: "PROFILE_CREATION_REQUIRED"; data: string }
  | { type: "AUTH_COMPLETE" }
  | { type: "AUTH_ERROR"; error: any }
  | { type: "GDL_SETUP_COMPLETE" }
  | { type: "GDL_SETUP_CANCELLED" }
  | { type: "SHOW_SEASONAL_MESSAGE" }
  | { type: "SHOW_SEASONAL_BUTTON" }
  | { type: "UPDATE_REFS"; refs: Partial<LoginMachineContext["refs"]> }
  | { type: "ANIMATION_COMPLETE" }
