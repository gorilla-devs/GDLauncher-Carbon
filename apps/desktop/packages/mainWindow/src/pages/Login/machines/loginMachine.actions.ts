import { assign, enqueueActions } from "xstate"
import type { LoginMachineContext } from "./loginMachine.types"

export const actions = {
  // State updates
  setTermsAccepted: assign({
    termsAccepted: true
  }),

  setTransitionForward: assign({
    transitionDirection: "forward" as const
  }),

  setTransitionBackward: assign({
    transitionDirection: "backward" as const
  }),

  setAuthFlowTypeBrowser: assign({
    authFlowType: 1 // AuthFlowType.Browser
  }),

  setAuthFlowTypeDeviceCode: assign({
    authFlowType: 2 // AuthFlowType.DeviceCode
  }),

  setDeviceCodeObject: assign({
    deviceCodeObject: ({ event }) => {
      if (event.type === "POLLING_CODE_RECEIVED") {
        return event.data
      }
      return null
    }
  }),

  setProfileAccessToken: assign({
    profileAccessToken: ({ event }) => {
      if (event.type === "PROFILE_CREATION_REQUIRED") {
        return event.data
      }
      return null
    }
  }),

  setHasGDLAccount: assign({
    hasGDLAccount: true
  }),

  setError: assign({
    error: ({ event }) => {
      if (event.type === "AUTH_ERROR") {
        return event.error?.message || "Unknown error"
      }
      return null
    }
  }),

  setLoadingButtonFalse: assign({
    loadingButton: false
  }),

  setLoadingButtonTrue: assign({
    loadingButton: true
  }),

  updateRefs: assign({
    refs: ({ context, event }) => {
      if (event.type === "UPDATE_REFS") {
        return { ...context.refs, ...event.refs }
      }
      return context.refs
    }
  }),

  // Seasonal
  showSeasonalMessage: assign({
    seasonalMessageVisible: true
  }),

  showSeasonalButton: assign({
    seasonalButtonVisible: true
  }),

  clearSeasonalState: assign({
    seasonalMessageVisible: false,
    seasonalButtonVisible: false
  }),

  // Animation triggers
  animateSidebarIn: ({ context }: { context: LoginMachineContext }) => {
    const { sidebar, video, loadingSpinner, backgroundBlur } = context.refs

    setTimeout(() => {
      sidebar?.animate(
        [{ transform: "translateX(-100%)" }, { transform: "translateX(0)" }],
        {
          duration: 300,
          delay: 200,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
          fill: "forwards"
        }
      )

      video?.animate(
        [{ transform: "translateX(0)" }, { transform: "translateX(15%)" }],
        {
          duration: 300,
          delay: 200,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
          fill: "forwards"
        }
      )

      loadingSpinner?.animate([{ opacity: 1 }, { opacity: 0 }], {
        duration: 300,
        easing: "linear",
        fill: "forwards"
      })

      backgroundBlur?.animate([{ opacity: 1 }, { opacity: 0 }], {
        duration: 500,
        easing: "linear",
        fill: "forwards"
      })
    }, 300)
  },

  hideLoadingOverlay: ({ context }: { context: LoginMachineContext }) => {
    const { loadingSpinner, backgroundBlur } = context.refs

    loadingSpinner?.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 300,
      easing: "linear",
      fill: "forwards"
    })

    backgroundBlur?.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: 500,
      easing: "linear",
      fill: "forwards"
    })
  },

  showBackButton: ({ context }: { context: LoginMachineContext }) => {
    const { backButton } = context.refs
    if (backButton) {
      backButton.animate(
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
  },

  hideBackButton: ({ context }: { context: LoginMachineContext }) => {
    const { backButton } = context.refs
    if (backButton) {
      backButton.animate(
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
  },

  // Seasonal timing sequence
  startSeasonalTimers: enqueueActions(({ enqueue }) => {
    enqueue.sendTo(
      ({ system }) => system.get("loginMachine"),
      { type: "SHOW_SEASONAL_MESSAGE" },
      { delay: 500 }
    )
    enqueue.sendTo(
      ({ system }) => system.get("loginMachine"),
      { type: "SHOW_SEASONAL_BUTTON" },
      { delay: 1000 }
    )
  }),

  // Logging
  logGDLCheckError: ({ event }: { event: any }) => {
    console.error("Failed to check GDL account:", event.error)
  },

  logLinkError: ({ event }: { event: any }) => {
    console.error("Failed to link GDL account:", event.error)
  }
}
