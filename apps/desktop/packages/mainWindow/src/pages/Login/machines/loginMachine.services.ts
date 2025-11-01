import { fromPromise } from "xstate"
import type { LoginMachineContext } from "./loginMachine.types"
import type { AccountEntry } from "@gd/core_module/bindings"
import type { CreateMutationResult } from "@tanstack/solid-query"
import { RSPCError } from "@rspc/client"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { rspc } from "@/utils/rspcClient"

type GlobalStoreContext = ReturnType<typeof useGlobalStore>
type RspcContext = ReturnType<typeof rspc.useContext>
type NavigationContext = ReturnType<typeof useGDNavigate>

// Define the exact shape of services that the machine expects
export interface LoginMachineServices {
  initializeLogin: ReturnType<
    typeof fromPromise<
      {
        accounts: AccountEntry[]
        termsAndPrivacyAccepted: boolean
        gdlAccountId: string | null
        isFirstLaunch: boolean
        activeUuid: string | null
      },
      Partial<LoginMachineContext>
    >
  >

  beginBrowserEnrollment: ReturnType<typeof fromPromise<void, void>>
  beginDeviceCodeEnrollment: ReturnType<typeof fromPromise<void, void>>
  cancelEnrollment: ReturnType<typeof fromPromise<void, void>>
  finalizeEnrollment: ReturnType<
    typeof fromPromise<{ activeUuid: string | null }, void>
  >

  checkGDLAccountStatus: ReturnType<
    typeof fromPromise<
      | { hasAccount: false; localGdlId: string | null }
      | {
          hasAccount: boolean
          accountData: any
          uuid: string
          localGdlId: string | null
        },
      LoginMachineContext
    >
  >

  linkExistingGDLAccount: ReturnType<
    typeof fromPromise<void, LoginMachineContext>
  >
  saveSkipGDLDecision: ReturnType<typeof fromPromise<void, void>>
  playWelcomeAnimation: ReturnType<
    typeof fromPromise<void, LoginMachineContext>
  >
}

// Define the custom actions type
export interface LoginMachineCustomActions {
  cancelEnrollmentAction: () => void
  navigateToReturnPath: ({ context }: { context: LoginMachineContext }) => void
  navigateToLibrary: () => void
  saveTermsAcceptanceAction: () => void
}

// Services will be provided from component with access to rspc context
// These are factory functions that return the actual service implementations

export const createServices = (deps: {
  rspcContext: RspcContext
  globalStore: GlobalStoreContext
  saveGdlAccountMutation: CreateMutationResult<
    null,
    RSPCError,
    string | null,
    unknown
  >
  enrollBeginMutation: CreateMutationResult<null, RSPCError, undefined, unknown>
  enrollBeginBrowserMutation: CreateMutationResult<
    null,
    RSPCError,
    boolean,
    unknown
  >
  enrollCancelMutation: CreateMutationResult<
    null,
    RSPCError,
    undefined,
    unknown
  >
  enrollFinalizeMutation: CreateMutationResult<
    null,
    RSPCError,
    undefined,
    unknown
  >
  settingsMutation: CreateMutationResult<null, RSPCError, any, unknown>
  navigator: NavigationContext
}): { services: LoginMachineServices; actions: LoginMachineCustomActions } => {
  const {
    rspcContext,
    globalStore,
    saveGdlAccountMutation,
    enrollBeginMutation,
    enrollBeginBrowserMutation,
    enrollCancelMutation,
    enrollFinalizeMutation,
    settingsMutation,
    navigator
  } = deps

  const services: LoginMachineServices = {
    initializeLogin: fromPromise(async () => {
      // Refetch settings
      await rspcContext.queryClient.refetchQueries({
        queryKey: ["settings.getSettings"]
      })

      const accounts = await rspcContext.client.query(["account.getAccounts"])

      return {
        accounts,
        termsAndPrivacyAccepted:
          globalStore.settings.data?.termsAndPrivacyAccepted ?? false,
        gdlAccountId: globalStore.settings.data?.gdlAccountId ?? null,
        isFirstLaunch: globalStore.settings.data?.isFirstLaunch ?? false,
        activeUuid: globalStore.currentlySelectedAccountUuid.data ?? null
      }
    }),

    beginBrowserEnrollment: fromPromise(async () => {
      await enrollBeginBrowserMutation.mutateAsync(true)
    }),

    beginDeviceCodeEnrollment: fromPromise(async () => {
      await enrollBeginMutation.mutateAsync(undefined)
    }),

    cancelEnrollment: fromPromise(async () => {
      try {
        await enrollCancelMutation.mutateAsync(undefined)
      } catch (_err) {
        console.log("Cancel failed (expected if no enrollment active)")
      }
    }),

    finalizeEnrollment: fromPromise(async () => {
      await enrollFinalizeMutation.mutateAsync(undefined)

      // Refetch accounts
      await Promise.all([
        rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getAccounts"]
        }),
        rspcContext.queryClient.refetchQueries({
          queryKey: ["account.getActiveUuid"]
        })
      ])

      const activeUuid = await rspcContext.client.query([
        "account.getActiveUuid"
      ])
      return { activeUuid }
    }),

    checkGDLAccountStatus: fromPromise(
      async ({ input }: { input: LoginMachineContext }) => {
        const { activeUuid, gdlAccountId } = input

        if (!activeUuid) {
          return { hasAccount: false, localGdlId: gdlAccountId }
        }

        // Verify account exists
        const accounts = await rspcContext.client.query(["account.getAccounts"])
        const accountExists = accounts.some(
          (acc: AccountEntry) => acc.uuid === activeUuid
        )

        if (!accountExists) {
          throw new Error(`Account ${activeUuid} not found`)
        }

        // Check cloud for GDL account
        try {
          const gdlAccount = await rspcContext.client.query([
            "account.peekGdlAccount",
            activeUuid
          ])

          return {
            hasAccount: !!gdlAccount,
            accountData: gdlAccount,
            uuid: activeUuid,
            localGdlId: gdlAccountId
          }
        } catch (error) {
          console.error("Failed to peek GDL account:", error)
          return { hasAccount: false, localGdlId: gdlAccountId }
        }
      }
    ),

    linkExistingGDLAccount: fromPromise(
      async ({ input }: { input: LoginMachineContext }) => {
        await saveGdlAccountMutation.mutateAsync(input.pendingGDLAccountUuid)
      }
    ),

    saveSkipGDLDecision: fromPromise(async () => {
      await saveGdlAccountMutation.mutateAsync("")
    }),

    playWelcomeAnimation: fromPromise(
      async ({ input }: { input: LoginMachineContext }) => {
        console.log(
          "[playWelcomeAnimation] Starting, isFirstLaunch:",
          input.isFirstLaunch
        )

        try {
          if (input.isFirstLaunch) {
            const {
              sidebar,
              video,
              backgroundBlur,
              welcomeToText,
              gdlauncherText
            } = input.refs

            // Sidebar slides out
            sidebar?.animate(
              [
                { transform: "translateX(0%)" },
                { transform: "translateX(-100%)" }
              ],
              { duration: 500, easing: "linear", fill: "forwards" }
            )

            // Video slides to center
            video?.animate(
              [
                { transform: "translateX(15%)" },
                { transform: "translateX(0%)" }
              ],
              { duration: 300, easing: "linear", fill: "forwards" }
            )

            // Background blur fades in
            backgroundBlur?.animate([{ opacity: 0 }, { opacity: 1 }], {
              duration: 500,
              delay: 350,
              easing: "linear",
              fill: "forwards"
            })

            // "Welcome to" text fades in
            welcomeToText?.animate([{ opacity: 0 }, { opacity: 1 }], {
              duration: 600,
              delay: 1100,
              easing: "linear",
              fill: "forwards"
            })

            // "GDLauncher" text fades in
            gdlauncherText?.animate([{ opacity: 0 }, { opacity: 1 }], {
              duration: 600,
              delay: 2300,
              easing: "linear",
              fill: "forwards"
            })

            // Wait for full sequence
            await new Promise((resolve) => setTimeout(resolve, 5000))
          }

          // Always navigate to library (whether animation played or not)
          console.log("[playWelcomeAnimation] Navigating to /library")
          navigator.navigate("/library")
          console.log("[playWelcomeAnimation] Navigation called")
        } catch (error) {
          console.error("[playWelcomeAnimation] Error:", error)
          // Still try to navigate even if animation fails
          navigator.navigate("/library")
        }
      }
    )
  }

  const actions: LoginMachineCustomActions = {
    // Fire-and-forget cancel enrollment (doesn't wait for result)
    cancelEnrollmentAction: () => {
      enrollCancelMutation.mutateAsync(undefined).catch((_err: RSPCError) => {
        console.log("Cancel failed (expected if no enrollment active)")
      })
    },

    // Navigate back to return path
    navigateToReturnPath: ({ context }: { context: LoginMachineContext }) => {
      if (context.returnPath) {
        navigator.navigate(context.returnPath)
      }
    },

    // Navigate to library directly
    navigateToLibrary: () => {
      console.log("[navigateToLibrary action] Navigating to /library")
      navigator.navigate("/library")
    },

    // Save terms acceptance (fire-and-forget)
    saveTermsAcceptanceAction: () => {
      // Backend automatically saves termsAndPrivacyAcceptedChecksum when this is called
      settingsMutation
        .mutateAsync({
          termsAndPrivacyAccepted: {
            Set: true
          }
        })
        .catch((err) => {
          console.error("Failed to save terms acceptance:", err)
        })
    }
  }

  return { services, actions }
}
