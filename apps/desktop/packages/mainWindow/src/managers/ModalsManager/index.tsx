import { useLocation, useSearchParams } from "@solidjs/router"
import {
  createContext,
  createSignal,
  createUniqueId,
  For,
  JSX,
  lazy,
  onCleanup,
  onMount,
  useContext
} from "solid-js"
import { Dynamic, Portal } from "solid-js/web"
import { useTransContext, TypedTFunction } from "@gd/i18n"
import { useGDNavigate } from "../NavigationManager"
import adSize from "@/utils/adhelper"
import { listenMemoryWarning } from "@/utils/memoryWarningBridge"
import { listenShutdownQuestion } from "@/utils/shutdownQuestion"
import { listenServerEula } from "@/utils/serverEulaBridge"
import { listenSignInExpired } from "@/utils/signInExpiredBridge"
import { listenJavaMismatch } from "@/utils/javaMismatchBridge"
import { javaMismatchPrompt } from "@/utils/javaIssues"
import { cleanupRunning } from "./modals/CacheCleanup/state"
import { shaderInstallRunning } from "./modals/ShaderLoaderSetup/state"
import {
  isChangingRuntimePath,
  runtimeRecoveryRequired
} from "./modals/ConfirmChangeRuntimePath/state"
import { resolvePreventClose, topModalBlocksClose } from "./preventClose"
import { createDialogFocus, type DialogFocusHandle } from "./dialogFocus"
import { escapeClosesModal } from "./dialogEscape"
import { MODAL_ROW_CLIP } from "@/themes/decorations"

export interface ModalProps {
  title: string
  noHeader?: boolean
  data?: any
}

type Hash = Record<
  string,
  {
    component: ((_props: ModalProps) => JSX.Element) & {
      preload: () => Promise<{ default: (_props: ModalProps) => JSX.Element }>
    }
    /**
     * When `true` (or the function returns `true`), the backdrop click and
     * side panel will not close the modal. Pass a function for modals whose
     * closability depends on internal state (e.g. "running" vs "idle" phase).
     */
    preventClose?: boolean | (() => boolean)
    /** Read when the modal renders, so the title follows a language switch. */
    title?: () => string
    noHeader?: boolean
  }
>

const getDefaultModals = (t: TypedTFunction) => ({
  privacyStatement: {
    component: lazy(() => import("./modals/PrivacyStatement")),
    title: () => t("modals:_trn_privacy_statement")
  },
  termsAndConditions: {
    component: lazy(() => import("./modals/TermsAndConditions")),
    title: () => t("modals:_trn_terms_and_conditions")
  },
  addManagedJava: {
    component: lazy(() => import("./modals/Java/AddManagedJava")),
    title: () => t("java:_trn_download_java")
  },
  // These name what they delete, so their titles come with their data.
  deleteJavaConfirm: {
    component: lazy(() => import("./modals/Java/DeleteJavaConfirm"))
  },
  removeUnusedJavasConfirm: {
    component: lazy(() => import("./modals/Java/RemoveUnusedJavasConfirm"))
  },
  deleteJavaProfileConfirm: {
    component: lazy(() => import("./modals/Java/DeleteJavaProfileConfirm"))
  },
  // Names the instance, so its title comes with its data too.
  javaIssue: {
    component: lazy(() => import("./modals/Java/JavaIssueModal"))
  },
  shaderLoaderSetup: {
    component: lazy(() => import("./modals/ShaderLoaderSetup")),
    title: () => t("modals:_trn_shader_loader_setup"),
    // Block backdrop close while the wizard is mid-install. Closing then
    // would tear down the polling loop driving sequential steps and leave
    // a half-installed loader/shader pair.
    preventClose: () => shaderInstallRunning()
  },
  instanceCreation: {
    component: lazy(() => import("./modals/InstanceCreation")),
    title: () => t("modals:_trn_new_instance")
  },
  exportInstance: {
    component: lazy(() => import("./modals/InstanceExport")),
    title: () => t("modals:_trn_export_instance")
  },
  modpack_version_update: {
    component: lazy(() => import("./modals/ModPackVersionUpdate")),
    title: () => t("modals:_trn_change_modpack_version")
  },
  unlock_confirmation: {
    component: lazy(() => import("./modals/Confirmation")),
    title: () => t("modals:_trn_unlock_instance")
  },
  unpair_confirmation: {
    component: lazy(() => import("./modals/Confirmation")),
    title: () => t("modals:_trn_unpair_instance")
  },
  notification: {
    component: lazy(() => import("./modals/Notification")),
    title: () => t("modals:_trn_notification")
  },
  confirmInstanceDeletion: {
    component: lazy(() => import("./modals/ConfirmInstanceDeletion")),
    title: () => t("modals:_trn_confirm_instance_deletion")
  },
  repairModpack: {
    component: lazy(() => import("./modals/RepairModpack")),
    title: () => t("modals:_trn_repair_modpack")
  },
  keptPackFiles: {
    component: lazy(() => import("./modals/KeptPackFiles")),
    title: () => t("modals:_trn_kept_files")
  },
  confirmBatchInstanceDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchInstanceDeletion")),
    title: () => t("modals:_trn_confirm_batch_instance_deletion")
  },
  confirmServerDeletion: {
    component: lazy(() => import("./modals/ConfirmServerDeletion")),
    title: () => t("modals:_trn_confirm_server_deletion")
  },
  confirmBatchServerDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchServerDeletion")),
    title: () => t("modals:_trn_confirm_batch_server_deletion")
  },
  serverEulaAcceptance: {
    component: lazy(() => import("./modals/ServerEulaAcceptance")),
    title: () => t("modals:_trn_server_eula_acceptance")
  },
  recoveredServerProcess: {
    component: lazy(() => import("./modals/RecoveredServerProcess")),
    // The dialog sets its own title for the action it confirms.
    title: () => t("modals:_trn_server_clear_record_title")
  },
  forceStopServer: {
    component: lazy(() => import("./modals/ForceStopServer")),
    title: () => t("modals:_trn_server_force_stop_title")
  },
  confirmBatchFolderDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchFolderDeletion")),
    // The dialog sets its own title for the action it confirms.
    title: () => t("instances:_trn_folder_remove_title")
  },
  confirmBatchMixedDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchMixedDeletion")),
    title: () => t("modals:_trn_confirm_batch_deletion")
  },
  ConfirmChangeRuntimePath: {
    component: lazy(() => import("./modals/ConfirmChangeRuntimePath")),
    title: () => t("modals:_trn_confirm_change_runtime_path"),
    // Backdrop and side-panel close are blocked while the migration is
    // running. Closing mid-flight would orphan files between old and new
    // runtime paths.
    preventClose: () => isChangingRuntimePath() || runtimeRecoveryRequired()
  },
  runtimeMoveNotice: {
    component: lazy(() => import("./modals/RuntimeMoveNotice")),
    title: () => t("java:_trn_runtime_recovery.moved_title"),
    // It may be the only list of files newer than their copy: a stray Escape
    // or backdrop click must not dismiss it, only OK or the close button.
    preventClose: true
  },
  runtimePathUnavailable: {
    component: lazy(() => import("./modals/RuntimePathUnavailable")),
    title: () => t("java:_trn_runtime_recovery.unavailable_title")
  },
  onBoarding: {
    component: lazy(() => import("./modals/OnBoarding")),
    // Names the dialog for screen readers, without its emoji; with
    // `noHeader`, it doesn't show.
    title: () =>
      t("onboarding:_trn_welcome_gdlauncher_title")
        .replace(/\p{Extended_Pictographic}/gu, "")
        .trim(),
    noHeader: true
  },
  whyAreAdsNeeded: {
    component: lazy(() => import("./modals/WhyAreAdsNeeded")),
    title: () => t("modals:_trn_why_are_ads_needed")
  },
  modsUpdater: {
    component: lazy(() => import("./modals/ModsUpdater")),
    title: () => t("modals:_trn_mods_updater")
  },
  javaProfileCreation: {
    component: lazy(() => import("./modals/JavaProfileCreationModal")),
    title: () => t("modals:_trn_create_java_profile")
  },
  windowCloseWarning: {
    component: lazy(() => import("./modals/WindowCloseWarning")),
    title: () => t("modals:_trn_confirm_quit")
  },
  changelogs: {
    component: lazy(() => import("./modals/Changelogs")),
    title: () => t("modals:_trn_welcome_new_version")
  },
  confirmGDLAccountDeletion: {
    component: lazy(() => import("./modals/ConfirmGDLAccountDeletion")),
    title: () => t("modals:_trn_confirm_account_deletion")
  },
  confirmMsWithGDLAccountRemoval: {
    component: lazy(() => import("./modals/ConfirmMsWithGDLAccountRemoval")),
    title: () => t("modals:_trn_confirm_account_removal")
  },
  accountExpired: {
    component: lazy(() => import("./modals/AccountExpired")),
    title: () => t("modals:_trn_account_expired")
  },
  changeGDLAccountRecoveryEmail: {
    component: lazy(() => import("./modals/ChangeGDLAccountRecoveryEmail")),
    title: () => t("modals:_trn_change_recovery_email")
  },
  editGDLProfile: {
    component: lazy(() => import("./modals/EditGDLProfile")),
    title: () => t("modals:_trn_edit_profile")
  },
  changeGDLAccountDisplayName: {
    component: lazy(() => import("./modals/ChangeGDLAccountDisplayName")),
    title: () => t("modals:_trn_change_display_name")
  },
  modDetails: {
    component: lazy(() => import("./modals/ModDetails")),
    title: () => t("modals:_trn_mod_details")
  },
  platformSelection: {
    component: lazy(() => import("./modals/PlatformSelection")),
    title: () => t("instances:_trn_select_platform")
  },
  duplicatedModsResolution: {
    component: lazy(() => import("./modals/DuplicatedModsResolution")),
    title: () => t("content:_trn_duplicated_mods_detected")
  },
  betaPrompt: {
    component: lazy(() => import("./modals/BetaPrompt")),
    title: () => t("modals:_trn_beta_prompt_title")
  },
  seasonalTheme: {
    component: lazy(() => import("./modals/SeasonalTheme")),
    noHeader: true
  },
  themeRemove: {
    component: lazy(() => import("./modals/ThemeRemove")),
    title: () => t("settings:_trn_theme_remove_title")
  },
  themeInstall: {
    component: lazy(() => import("./modals/ThemeInstall")),
    title: () => t("settings:_trn_install_theme")
  },
  shareInstance: {
    component: lazy(() => import("./modals/ShareInstance")),
    title: () => t("instances:_trn_instance_share.title")
  },
  myShares: {
    component: lazy(() => import("./modals/MyShares")),
    title: () => t("instances:_trn_my_shares.title")
  },
  editShare: {
    component: lazy(() => import("./modals/EditShare")),
    title: () => t("instances:_trn_my_shares.edit")
  },
  sharePreview: {
    component: lazy(() => import("./modals/SharePreview")),
    title: () => t("instances:_trn_share_preview.title")
  },
  report: {
    component: lazy(() => import("./modals/Report")),
    title: () => t("instances:_trn_report.title")
  },
  accountBanned: {
    component: lazy(() => import("./modals/AccountBanned")),
    // Names the dialog for screen readers; with `noHeader`, it doesn't show.
    title: () => t("accounts:_trn_banned_dialog_title"),
    preventClose: true,
    noHeader: true
  },
  requiresGdlAccount: {
    component: lazy(() => import("./modals/RequiresGdlAccountModal")),
    title: () => t("accounts:_trn_requires_gdl_account")
  },
  insufficientMemory: {
    component: lazy(() => import("./modals/InsufficientMemory")),
    title: () => t("java:_trn_insufficient_memory_title")
  },
  confirmAddonDeletion: {
    component: lazy(() => import("./modals/ConfirmAddonDeletion")),
    title: () => t("instances:_trn_confirm_addon_deletion_title")
  },
  confirmWorldDeletion: {
    component: lazy(() => import("./modals/ConfirmWorldDeletion"))
  },
  // Its title comes with the question, in the modal's data.
  confirmChoice: {
    component: lazy(() => import("./modals/ConfirmChoice"))
  },
  serverCreation: {
    component: lazy(() => import("./modals/ServerCreation")),
    title: () => "New Server"
  },
  serverRename: {
    component: lazy(() => import("./modals/ServerRename")),
    title: () => t("modals:_trn_server_rename")
  },
  cacheCleanup: {
    component: lazy(() => import("./modals/CacheCleanup")),
    // Block backdrop/side-panel close only while a cleanup is actively
    // running; closing mid-VACUUM reveals a frozen UI (connection_limit=1
    // serializes every other DB query) and the modal can't be reopened onto
    // the in-flight task. All other phases close normally.
    preventClose: () => cleanupRunning(),
    title: () => t("modals:_trn_cache_cleanup_title")
  }
})

type ModalName = keyof ReturnType<typeof getDefaultModals>

interface Modal {
  name: ModalName
  url?: string
}

interface Context {
  openModal: (_modal: Modal, _data?: any) => void
  closeModal: (_name?: ModalName) => void
  /** Closes every open modal, for when the page under them is being left. */
  closeAllModals: () => void
  hasOpenModals: () => boolean
  /** The top modal can't be closed right now (its `preventClose`). */
  isTopModalLocked: () => boolean
}

export interface StackEntry {
  name: ModalName
  data: any
  /** Live `preventClose` read for this modal instance, set by its mounted
   *  ModalLayout (see `ModalStackEntryContext` below) and cleared on
   *  unmount. Escape/backdrop OR this together with the static registry's
   *  `preventClose` — either source can block the close. */
  preventCloseAccessor?: () => boolean
  /** This modal's place in the dialog focus trap, set once its element
   *  exists. */
  focusHandle?: DialogFocusHandle
}

type Stack = StackEntry[]

const ModalsContext = createContext<Context>()

export interface ModalStackEntryApi {
  /** The dialog's name, for its identity hook (`modal.<name>`). */
  name: string
  registerPreventClose: (_accessor: () => boolean) => void
  unregisterPreventClose: () => void
  /** Close only this live entry while it is still the current top modal. */
  closeIfCurrentTop: () => boolean
  /** Remove only this live entry, including while another modal is above it. */
  closeOwn: () => boolean
  /** Closes this modal, wherever it is in the stack. For work that ends
   *  after another modal may have opened on top. */
  close: () => void
  /** The id for ModalLayout's title, which labels the dialog. */
  titleId: string
  /** ModalLayout says whether its header shows the title. */
  setTitleShown: (_shown: boolean) => void
}

// Lets the ModalLayout rendered for a given stack entry register its own
// live `preventClose` prop so Escape/backdrop can see it — without this,
// only the static registry's `preventClose` (keyed by modal name) reached
// those two close paths, while a modal-instance-local prop only ever guarded
// ModalLayout's own header close button.
const ModalStackEntryContext = createContext<ModalStackEntryApi>()

export const useModalStackEntry = () => useContext(ModalStackEntryContext)

export const ModalProvider = (props: { children: JSX.Element }) => {
  const [t] = useTransContext()
  const defaultModals = getDefaultModals(t)
  const navigator = useGDNavigate()
  const location = useLocation()
  const queryParams = () => location.search as ModalName
  const urlSearchParams = () => new URLSearchParams(queryParams())
  const [modalStack, setModalStack] = createSignal<Stack>([])
  let modalPortalRef: HTMLDivElement | undefined
  const dialogFocus = createDialogFocus(document)
  // Hiding the overlay waits for its fade; a modal opened meanwhile (close
  // one, open the next) cancels it, or the new modal would be hidden.
  let hideOverlayTimer: ReturnType<typeof setTimeout> | undefined

  const [_searchParams, setSearchParams] = useSearchParams()

  /** Removes the stack entry `pick` points at, if any. */
  const closeAt = (pick: (_stack: Stack) => number) => {
    setModalStack((currentStack) => {
      const newStack = currentStack.slice()
      const indexToRemove = pick(currentStack)

      if (indexToRemove >= 0) {
        // Before the modal's own cleanup runs, so the trap no longer holds
        // focus in it when an `onClosed` there moves focus to the opener.
        currentStack[indexToRemove].focusHandle?.close()
        newStack.splice(indexToRemove, 1)

        // The URL stores the modal stack as `m[1]=name1`, `m[2]=name2`, ...
        // (1-indexed). After mutating the stack we must rebuild every `m[k]`
        // so positions reflect the new stack — closing a non-top modal would
        // otherwise leave stale values pointing at the removed modal.
        const newParams: Record<string, string | null> = {}
        for (const key of Object.keys(Object.fromEntries(urlSearchParams()))) {
          if (/^m\[\d+\]$/.test(key)) {
            newParams[key] = null
          }
        }
        for (let i = 0; i < newStack.length; i++) {
          newParams[`m[${i + 1}]`] = newStack[i].name
        }

        setSearchParams(newParams)
      }

      return newStack
    })

    if (modalStack().length === 0) {
      const overlay = document.getElementById("overlay")!
      // Only hide the overlay if nothing else (e.g. an expanded folder) is
      // using it. The folder portal also renders into #overlay, so hiding it
      // here would close the folder too.
      const hasOtherContent = Array.from(overlay.children).some(
        (child) =>
          !(modalPortalRef && child.contains(modalPortalRef)) &&
          child.childNodes.length > 0
      )
      if (!hasOtherContent) {
        overlay.style.opacity = "0"
        clearTimeout(hideOverlayTimer)
        hideOverlayTimer = setTimeout(() => {
          overlay.style.display = "none"
        }, 100) // Wait for transition to complete
      }
    }
  }

  const closeModal = (name?: ModalName) =>
    closeAt((stack) =>
      name ? stack.findIndex((modal) => modal.name === name) : stack.length - 1
    )

  // Whether the given stack entry currently blocks Escape/backdrop close —
  // see `resolvePreventClose` for how the registry and the live ModalLayout
  // accessor are combined.
  const shouldPreventModalClose = (entry: StackEntry) =>
    resolvePreventClose((defaultModals as Hash)[entry.name].preventClose, entry)

  onMount(() => {
    const cleanupMemory = listenMemoryWarning((data) => {
      manager.openModal({ name: "insufficientMemory" }, data)
    })
    const cleanupEula = listenServerEula((data) => {
      manager.openModal({ name: "serverEulaAcceptance" }, data)
    })
    // Play could not renew the account's sign-in: asked whether to start
    // the game anyway.
    const cleanupSignIn = listenSignInExpired((data) => {
      manager.openModal({ name: "accountExpired" }, { id: data.instance_id })
    })
    // Asked before a restart or an update install stops a local server or
    // cancels an install.
    const cleanupShutdownQuestion = listenShutdownQuestion((question) => {
      manager.openModal({ name: "windowCloseWarning" }, question)
    })
    // PLAY on a Java that doesn't fit: the crash help dialog asks first.
    const cleanupJavaMismatch = listenJavaMismatch((data) => {
      manager.openModal({ name: "javaIssue" }, javaMismatchPrompt(data))
    })

    // Escape closes the top modal, mirroring a backdrop click: skipped when the
    // modal opts out of closing (preventClose), and when `escapeClosesModal`
    // leaves the Escape to something else (an open popup, a composing text
    // field, a control that cancelled it).
    const onKeyDown = (e: KeyboardEvent) => {
      if (!escapeClosesModal(e, document)) {
        return
      }
      const stack = modalStack()
      if (stack.length === 0) {
        return
      }
      const top = stack[stack.length - 1]
      if (shouldPreventModalClose(top)) {
        return
      }
      e.preventDefault()
      closeModal()
    }
    document.addEventListener("keydown", onKeyDown)
    dialogFocus.connect()

    onCleanup(() => {
      cleanupMemory()
      cleanupEula()
      cleanupSignIn()
      cleanupShutdownQuestion()
      cleanupJavaMismatch()
      document.removeEventListener("keydown", onKeyDown)
      dialogFocus.disconnect()
    })
  })

  const manager = {
    openModal: (modal: Modal, data: any) => {
      const overlay = document.getElementById("overlay")!
      clearTimeout(hideOverlayTimer)
      overlay.style.display = "flex"
      overlay.style.transition = "opacity 100ms ease-spring"
      overlay.style.opacity = "0" // Set initial opacity to 0
      setTimeout(() => (overlay.style.opacity = "1"), 10) // Transition to opacity 1
      setModalStack((currentStack) => [
        ...currentStack,
        { name: modal.name, data }
      ])

      // Update URL params
      if (modal.url) {
        const url = new URLSearchParams(modal.url)

        url.append(`m[${modalStack().length}]`, modal.name)

        const decodedParamString = decodeURIComponent(url.toString())
        navigator.navigate(decodedParamString.replace("=&", "?"))
      } else {
        setSearchParams({
          [`m[${modalStack().length}]`]: modal.name
        })
      }
    },
    closeModal,
    closeAllModals: () => {
      while (modalStack().length > 0) closeModal()
    },
    hasOpenModals: () => modalStack().length > 0,
    isTopModalLocked: () =>
      topModalBlocksClose(
        modalStack(),
        (name) => (defaultModals as Hash)[name]?.preventClose
      )
  }

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay")!}>
        <div ref={modalPortalRef} class="h-screen w-screen">
          <For each={modalStack()}>
            {(modal, index) => {
              const ModalComponent = defaultModals[modal.name].component
              const noHeader =
                (defaultModals as Hash)[modal.name].noHeader || false
              const title = () =>
                (defaultModals as Hash)[modal.name].title?.() || ""
              const titleId = createUniqueId()
              const [titleShown, setTitleShown] = createSignal(false)

              // Bound to this stack entry's own object (stable for the
              // entry's lifetime — `<For>` only calls this mapper once per
              // item), so the mounted ModalLayout's registration can never
              // land on a different modal instance.
              let entryLive = true
              onCleanup(() => {
                entryLive = false
              })
              const stackEntryApi: ModalStackEntryApi = {
                name: modal.name,
                registerPreventClose: (accessor) => {
                  modal.preventCloseAccessor = accessor
                },
                unregisterPreventClose: () => {
                  modal.preventCloseAccessor = undefined
                },
                closeIfCurrentTop: () => {
                  const stack = modalStack()
                  if (!entryLive || stack[stack.length - 1] !== modal) {
                    return false
                  }
                  closeModal()
                  return true
                },
                closeOwn: () => {
                  if (!entryLive || !modalStack().includes(modal)) return false
                  closeAt((stack) => stack.indexOf(modal))
                  return true
                },
                close: () => closeAt((stack) => stack.indexOf(modal)),
                titleId,
                setTitleShown
              }
              // `closeAt` closes it first; this covers the provider going.
              onCleanup(() => modal.focusHandle?.close())

              return (
                <div class="absolute inset-0 flex h-screen w-screen">
                  <div
                    class="z-999 relative flex h-full grow items-center justify-center"
                    style={MODAL_ROW_CLIP}
                    onMouseDown={() => {
                      if (!shouldPreventModalClose(modal)) {
                        closeModal()
                      }
                    }}
                  >
                    <div
                      ref={(el) => (modal.focusHandle = dialogFocus.open(el))}
                      role="dialog"
                      aria-modal="true"
                      // The header's title, or without one the registry's.
                      aria-labelledby={titleShown() ? titleId : undefined}
                      aria-label={
                        titleShown() ? undefined : title() || undefined
                      }
                      tabindex="-1"
                      style={{ "z-index": `${index() + 1}` }}
                      onMouseDown={(e) => e.stopPropagation()}
                      class="animate-modalEnter outline-none"
                    >
                      <ModalStackEntryContext.Provider value={stackEntryApi}>
                        <Dynamic
                          component={ModalComponent}
                          data={modal.data}
                          noHeader={noHeader}
                          title={title()}
                        />
                      </ModalStackEntryContext.Provider>
                    </div>
                    <div
                      data-gdl="modal.backdrop"
                      class="bg-darkSlate-900 absolute inset-0 opacity-95 transition-opacity duration-100"
                    />
                  </div>

                  {/* The dim over the ad column's backdrop, in the same colour
                      as the page's (`modal.backdrop`). The ad is a separate
                      layer on top of it. */}
                  <div
                    data-gdl="modal.column-backdrop"
                    class="text-lightSlate-50 z-999 bg-darkSlate-900 h-screen origin-center place-items-center opacity-95 transition-opacity duration-100"
                    style={{
                      width: `${adSize.width}px`
                    }}
                    onMouseDown={() => {
                      if (!shouldPreventModalClose(modal)) {
                        closeModal()
                      }
                    }}
                  />
                </div>
              )
            }}
          </For>
        </div>
      </Portal>
    </ModalsContext.Provider>
  )
}

export const useModal = () => {
  return useContext(ModalsContext)
}
