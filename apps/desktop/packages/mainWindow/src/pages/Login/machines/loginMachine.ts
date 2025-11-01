import { setup, assign } from "xstate"
import type {
  LoginMachineContext,
  LoginMachineEvents,
  AuthFlowType,
  LoginStep
} from "./loginMachine.types"
import type {
  LoginMachineServices,
  LoginMachineCustomActions
} from "./loginMachine.services"
import { guards } from "./loginMachine.guards"
import { actions as baseActions } from "./loginMachine.actions"

export const createLoginMachine = (servicesAndActions: {
  services: LoginMachineServices
  actions: LoginMachineCustomActions
}) => {
  const { services, actions: customActions } = servicesAndActions

  return setup({
    types: {
      context: {} as LoginMachineContext,
      events: {} as LoginMachineEvents,
      input: {} as Partial<LoginMachineContext>
    },
    guards,
    // @ts-expect-error - XState v5's setup() expects actions defined inline for proper type inference.
    // Spreading baseActions and customActions from separate files breaks the inference chain.
    // This is a known XState v5 limitation with dynamic action composition.
    actions: {
      ...baseActions,
      ...customActions
    },
    // @ts-expect-error - XState v5's setup() expects actors: Record<string, UnknownActorLogic>.
    // Our LoginMachineServices uses explicit fromPromise<TOutput, TInput> types which are
    // more specific than UnknownActorLogic, causing a type mismatch. This provides better
    // type safety at function boundaries while maintaining XState compatibility.
    actors: services
  }).createMachine({
    id: "loginMachine",
    context: ({ input }) => ({
      // User data
      activeUuid: input.activeUuid ?? null,
      accounts: input.accounts ?? [],

      // GDL Account state
      hasGDLAccount: input.hasGDLAccount ?? false,
      foundExistingGDLAccount: false,
      foundGDLAccountData: null,
      pendingGDLAccountUuid: null,
      gdlAccountId: input.gdlAccountId ?? null,

      // Auth flow state
      authFlowType: 0 as AuthFlowType, // AuthFlowType.None
      deviceCodeObject: null,
      profileAccessToken: null,
      enrollmentStatus: null,

      // UI state
      currentStep: 1 as LoginStep, // LoginStep.Welcome
      transitionDirection: "forward" as const,
      loadingButton: false,
      termsAccepted: false,

      // Profile creation state
      profileCreationValid: false,
      profileCreationPending: false,
      profileCreationSubmit: null,

      // Seasonal state
      currentOccasion: input.currentOccasion ?? null,
      seasonalMessageVisible: false,
      seasonalButtonVisible: false,

      // Navigation params
      isAddingAccount: input.isAddingAccount ?? false,
      isAddingMicrosoftAccount: input.isAddingMicrosoftAccount ?? false,
      isAddingGdlAccount: input.isAddingGdlAccount ?? false,
      returnPath: input.returnPath ?? null,

      // Settings
      isFirstLaunch: input.isFirstLaunch ?? false,
      termsAndPrivacyAccepted: input.termsAndPrivacyAccepted ?? false,
      reducedMotion: input.reducedMotion ?? false,

      // DOM refs
      refs: {},

      // Error state
      error: null
    }),
    initial: "initializing",
    states: {
      initializing: {
        invoke: {
          src: "initializeLogin",
          input: ({ context }: any) => context,
          onDone: {
            target: "determiningInitialState",
            actions: assign({
              termsAndPrivacyAccepted: ({ event }: any) =>
                event.output.termsAndPrivacyAccepted,
              gdlAccountId: ({ event }: any) => event.output.gdlAccountId,
              isFirstLaunch: ({ event }: any) => event.output.isFirstLaunch,
              activeUuid: ({ event }: any) => event.output.activeUuid,
              accounts: ({ event }: any) => event.output.accounts
            })
          }
        }
      },

      determiningInitialState: {
        always: [
          {
            guard: "isAddingMicrosoftFromSettings",
            target: "authFlow",
            actions: "animateSidebarIn"
          },
          {
            guard: "isAddingGdlFromSettings",
            target: "checkingExistingAccount"
          },
          {
            guard: "hasExistingAccount",
            target: "checkingExistingAccount"
          },
          {
            guard: "needsTermsAcceptance",
            target: "onboarding",
            actions: "animateSidebarIn"
          },
          {
            target: "authFlow",
            actions: "animateSidebarIn"
          }
        ]
      },

      checkingExistingAccount: {
        invoke: {
          src: "checkGDLAccountStatus",
          input: ({ context }: any) => context,
          onDone: [
            {
              guard: ({ event }: any) =>
                event.output.hasAccount &&
                event.output.localGdlId === event.output.uuid,
              target: "decidingPostAuth",
              actions: assign({ hasGDLAccount: true })
            },
            {
              guard: ({ event }: any) => event.output.localGdlId === "",
              target: "decidingPostAuth",
              actions: assign({ hasGDLAccount: false })
            },
            {
              guard: ({ event }: any) => event.output.hasAccount,
              target: "complete",
              actions: [
                assign({
                  foundExistingGDLAccount: true,
                  foundGDLAccountData: ({ event }: any) =>
                    event.output.accountData,
                  pendingGDLAccountUuid: ({ event }: any) => event.output.uuid
                }),
                "animateSidebarIn"
              ]
            },
            {
              target: "complete",
              actions: ["animateSidebarIn"]
            }
          ],
          onError: {
            target: "complete",
            actions: ["logGDLCheckError", "animateSidebarIn"]
          }
        }
      },

      decidingPostAuth: {
        always: [
          {
            guard: "shouldReturnToSettings",
            actions: "navigateToReturnPath"
          },
          {
            guard: "shouldShowSeasonalSplash",
            target: "seasonalSplash",
            actions: ["hideLoadingOverlay", "startSeasonalTimers"]
          },
          {
            target: "navigatingToLibrary"
          }
        ]
      },

      seasonalSplash: {
        entry: [
          () => {
            console.log("[seasonalSplash] Entered seasonal splash state")
          },
          "hideLoadingOverlay"
        ],
        exit: () => {
          console.log("[seasonalSplash] Exiting seasonal splash state")
        },
        on: {
          SHOW_SEASONAL_MESSAGE: {
            actions: "showSeasonalMessage"
          },
          SHOW_SEASONAL_BUTTON: {
            actions: "showSeasonalButton"
          },
          CONTINUE_SEASONAL: {
            target: "navigatingToLibrary",
            actions: "clearSeasonalState"
          }
        },
        after: {
          3000: {
            target: "navigatingToLibrary",
            actions: [
              () => {
                console.log(
                  "[seasonalSplash] 3 second timeout fired, transitioning to navigatingToLibrary"
                )
              },
              "clearSeasonalState"
            ]
          }
        }
      },

      onboarding: {
        initial: "welcome",
        states: {
          welcome: {
            on: {
              CONTINUE: {
                target: "termsAndPrivacy",
                actions: "setTransitionForward"
              }
            }
          },

          termsAndPrivacy: {
            on: {
              ACCEPT_TERMS: {
                actions: "setTermsAccepted"
              },
              CONTINUE: {
                guard: "termsAccepted",
                target: "#loginMachine.authFlow",
                actions: ["setTransitionForward", "saveTermsAcceptanceAction"]
              },
              BACK: {
                target: "welcome",
                actions: "setTransitionBackward"
              }
            }
          }
        }
      },

      authFlow: {
        initial: "authMethod",
        states: {
          navigatingBack: {
            entry: "navigateToReturnPath",
            type: "final"
          },

          authMethod: {
            on: {
              SELECT_BROWSER_AUTH: {
                target: "enrolling.browser",
                actions: [
                  "setTransitionForward",
                  "setAuthFlowTypeBrowser",
                  "setLoadingButtonTrue"
                ]
              },
              SELECT_DEVICE_CODE: {
                target: "enrolling.deviceCode",
                actions: [
                  "setTransitionForward",
                  "setAuthFlowTypeDeviceCode",
                  "setLoadingButtonTrue"
                ]
              },
              BACK: [
                {
                  guard: "isAddingFromSettings",
                  target: "navigatingBack",
                  actions: "setTransitionBackward"
                },
                {
                  target: "#loginMachine.onboarding.termsAndPrivacy",
                  actions: "setTransitionBackward"
                }
              ]
            }
          },

          enrolling: {
            initial: "idle",
            states: {
              idle: {},

              browser: {
                invoke: {
                  src: "beginBrowserEnrollment",
                  onDone: {
                    target: "waitingForBrowser"
                  },
                  onError: {
                    target: "failed",
                    actions: "setError"
                  }
                }
              },

              deviceCode: {
                invoke: {
                  src: "beginDeviceCodeEnrollment",
                  onDone: {
                    target: "pollingCode"
                  },
                  onError: {
                    target: "failed",
                    actions: "setError"
                  }
                }
              },

              waitingForBrowser: {
                entry: "setLoadingButtonFalse",
                on: {
                  BROWSER_AUTH_READY: {
                    actions: assign({
                      enrollmentStatus: ({ event }) => ({
                        waitingForBrowser: {
                          auth_url: event.data.authUrl,
                          redirect_uri: event.data.redirectUri,
                          expires_at: event.data.expiresAt
                        }
                      })
                    })
                  },
                  PROFILE_CREATION_REQUIRED: {
                    target: "profileCreation",
                    actions: ["setTransitionForward", "setProfileAccessToken"]
                  },
                  AUTH_COMPLETE: {
                    target: "#loginMachine.completingAuth",
                    actions: "setTransitionForward"
                  },
                  AUTH_ERROR: {
                    target: "failed",
                    actions: "setError"
                  },
                  SWITCH_TO_DEVICE_CODE: {
                    target: "deviceCode",
                    actions: "setTransitionForward"
                  },
                  BACK: {
                    target: "#loginMachine.authFlow.authMethod",
                    actions: ["setTransitionBackward", "cancelEnrollmentAction"]
                  }
                }
              },

              pollingCode: {
                entry: "setLoadingButtonFalse",
                on: {
                  POLLING_CODE_RECEIVED: {
                    actions: "setDeviceCodeObject"
                  },
                  PROFILE_CREATION_REQUIRED: {
                    target: "profileCreation",
                    actions: ["setTransitionForward", "setProfileAccessToken"]
                  },
                  AUTH_COMPLETE: {
                    target: "#loginMachine.completingAuth",
                    actions: "setTransitionForward"
                  },
                  AUTH_ERROR: {
                    target: "failed",
                    actions: "setError"
                  },
                  SWITCH_TO_BROWSER: {
                    target: "browser",
                    actions: "setTransitionForward"
                  },
                  BACK: {
                    target: "#loginMachine.authFlow.authMethod",
                    actions: ["setTransitionBackward", "cancelEnrollmentAction"]
                  }
                }
              },

              profileCreation: {
                on: {
                  CREATE_PROFILE: {
                    target: "#loginMachine.completingAuth",
                    actions: "setTransitionForward"
                  },
                  BACK: {
                    target: "#loginMachine.authFlow.authMethod",
                    actions: ["setTransitionBackward", "cancelEnrollmentAction"]
                  }
                }
              },

              failed: {
                on: {
                  RETRY_AUTH: {
                    target: "idle",
                    actions: "setTransitionForward"
                  },
                  BACK: {
                    target: "#loginMachine.authFlow.authMethod",
                    actions: "setTransitionBackward"
                  }
                }
              }
            }
          }
        }
      },

      completingAuth: {
        invoke: {
          src: "finalizeEnrollment",
          onDone: {
            target: "checkingGDLAccount",
            actions: assign({
              activeUuid: ({ event }) => event.output.activeUuid
            })
          },
          onError: {
            target: "authFlow.enrolling.failed",
            actions: "setError"
          }
        }
      },

      checkingGDLAccount: {
        invoke: {
          src: "checkGDLAccountStatus",
          input: ({ context }: any) => context,
          onDone: [
            {
              guard: "shouldReturnToSettings",
              actions: { type: "navigateToReturnPath" }
            },
            {
              guard: ({ event }: any) =>
                event.output.hasAccount &&
                event.output.localGdlId === event.output.uuid,
              target: "decidingPostAuth",
              actions: assign({ hasGDLAccount: true })
            },
            {
              guard: ({ event }: any) => event.output.localGdlId === "",
              target: "decidingPostAuth"
            },
            {
              guard: ({ event }: any) => event.output.hasAccount,
              target: "complete",
              actions: assign({
                foundExistingGDLAccount: true,
                foundGDLAccountData: ({ event }: any) =>
                  event.output.accountData,
                pendingGDLAccountUuid: ({ event }: any) => event.output.uuid
              })
            },
            {
              target: "complete"
            }
          ],
          onError: {
            target: "complete",
            actions: "logGDLCheckError"
          }
        }
      },

      complete: {
        initial: "idle",
        states: {
          idle: {},
          gdlSetup: {}
        },
        on: {
          SETUP_GDL_ACCOUNT: {
            target: ".gdlSetup",
            actions: "setTransitionForward"
          },
          GDL_SETUP_COMPLETE: {
            target: "decidingPostAuth",
            actions: ["setTransitionForward", "setHasGDLAccount"]
          },
          GDL_SETUP_CANCELLED: {
            target: ".idle",
            actions: "setTransitionBackward"
          },
          LINK_GDL_ACCOUNT: {
            target: "linkingGDLAccount",
            actions: "setTransitionForward"
          },
          SKIP_GDL_ACCOUNT: {
            target: "savingSkipDecision",
            actions: "setTransitionForward"
          },
          BACK: [
            {
              guard: "shouldReturnToSettings",
              actions: ["setTransitionBackward", "navigateToReturnPath"]
            },
            {
              target: "savingSkipDecision",
              actions: "setTransitionBackward"
            }
          ]
        }
      },

      savingSkipDecision: {
        invoke: {
          src: "saveSkipGDLDecision",
          onDone: {
            target: "decidingPostAuth"
          }
        }
      },

      linkingGDLAccount: {
        invoke: {
          src: "linkExistingGDLAccount",
          input: ({ context }: any) => context,
          onDone: {
            target: "decidingPostAuth",
            actions: "setHasGDLAccount"
          },
          onError: {
            target: "complete",
            actions: "logLinkError"
          }
        }
      },

      navigatingToLibrary: {
        entry: [
          () => {
            console.log(
              "[navigatingToLibrary] Entered navigatingToLibrary state"
            )
          },
          "navigateToLibrary"
        ],
        type: "final"
      }
    },
    on: {
      UPDATE_REFS: {
        actions: "updateRefs"
      }
    }
  })
}
