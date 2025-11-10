import { createContext, useContext, type JSX } from "solid-js"
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
import type { FlowController, AuthFlowConfig, AccountEntry } from "./types"
import { createFlowController } from "./FlowController"
import type { rspc } from "@/utils/rspcClient"

type RSPCContext = ReturnType<typeof rspc.useContext>

/**
 * Flow Context
 * Provides access to the flow controller throughout the auth flow
 */
const FlowContext = createContext<FlowController>()

export interface FlowProviderProps {
  children: JSX.Element
  config: AuthFlowConfig
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
  createProfileMutation: CreateMutationResult<null, RSPCError, FECreateProfile>
  enrollResumeMutation: CreateMutationResult<null, RSPCError, undefined>
  accountsQuery: CreateQueryResult<AccountEntry[], RSPCError>
  gdlAccountQuery: CreateQueryResult<FEGDLAccount | null, RSPCError>
}

/**
 * FlowProvider
 * Creates and provides the flow controller instance
 */
export function FlowProvider(props: FlowProviderProps) {
  const controller = createFlowController(props.config, {
    rspcContext: props.rspcContext,
    settingsMutation: props.settingsMutation,
    saveGdlAccountMutation: props.saveGdlAccountMutation,
    enrollBeginMutation: props.enrollBeginMutation,
    enrollBeginBrowserMutation: props.enrollBeginBrowserMutation,
    enrollCancelMutation: props.enrollCancelMutation,
    usernameAvailabilityMutation: props.usernameAvailabilityMutation,
    createProfileMutation: props.createProfileMutation,
    enrollResumeMutation: props.enrollResumeMutation,
    accountsQuery: props.accountsQuery,
    gdlAccountQuery: props.gdlAccountQuery
  })

  return (
    <FlowContext.Provider value={controller}>
      {props.children}
    </FlowContext.Provider>
  )
}

/**
 * useFlow hook
 * Access the flow controller from any component
 */
export function useFlow(): FlowController {
  const context = useContext(FlowContext)
  if (!context) {
    throw new Error("useFlow must be used within FlowProvider")
  }
  return context
}
