import { useLocation, useSearchParams } from "@solidjs/router"
import {
  createContext,
  createSignal,
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
import { listenServerEula } from "@/utils/serverEulaBridge"
import { cleanupRunning } from "./modals/CacheCleanup/state"
import { shaderInstallRunning } from "./modals/ShaderLoaderSetup/state"
import { isChangingRuntimePath } from "./modals/ConfirmChangeRuntimePath/state"
import { resolvePreventClose } from "./preventClose"

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
    title?: string
    noHeader?: boolean
  }
>

const getDefaultModals = (t: TypedTFunction) => ({
  privacyStatement: {
    component: lazy(() => import("./modals/PrivacyStatement")),
    title: t("modals:_trn_privacy_statement")
  },
  termsAndConditions: {
    component: lazy(() => import("./modals/TermsAndConditions")),
    title: t("modals:_trn_terms_and_conditions")
  },
  addManagedJava: {
    component: lazy(() => import("./modals/Java/AddManagedJava")),
    title: t("modals:_trn_add_java_version")
  },
  addCustomJava: {
    component: lazy(() => import("./modals/Java/AddCustomJava")),
    title: t("modals:_trn_add_java_version")
  },
  javaSetup: {
    component: lazy(() => import("./modals/Java/JavaSetup")),
    title: t("modals:_trn_java_setup")
  },
  shaderLoaderSetup: {
    component: lazy(() => import("./modals/ShaderLoaderSetup")),
    title: t("modals:_trn_shader_loader_setup"),
    // Block backdrop close while the wizard is mid-install. Closing then
    // would tear down the polling loop driving sequential steps and leave
    // a half-installed loader/shader pair.
    preventClose: () => shaderInstallRunning()
  },
  instanceCreation: {
    component: lazy(() => import("./modals/InstanceCreation")),
    title: t("modals:_trn_new_instance")
  },
  exportInstance: {
    component: lazy(() => import("./modals/InstanceExport")),
    title: t("modals:_trn_export_instance")
  },
  modpack_version_update: {
    component: lazy(() => import("./modals/ModPackVersionUpdate")),
    title: t("modals:_trn_change_modpack_version")
  },
  unlock_confirmation: {
    component: lazy(() => import("./modals/Confirmation")),
    title: t("modals:_trn_unlock_instance")
  },
  unpair_confirmation: {
    component: lazy(() => import("./modals/Confirmation")),
    title: t("modals:_trn_unpair_instance")
  },
  notification: {
    component: lazy(() => import("./modals/Notification")),
    title: t("modals:_trn_notification")
  },
  confirmInstanceDeletion: {
    component: lazy(() => import("./modals/ConfirmInstanceDeletion")),
    title: t("modals:_trn_confirm_instance_deletion")
  },
  repairModpack: {
    component: lazy(() => import("./modals/RepairModpack")),
    title: t("modals:_trn_repair_modpack")
  },
  confirmBatchInstanceDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchInstanceDeletion")),
    title: t("modals:_trn_confirm_batch_instance_deletion")
  },
  confirmBatchServerDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchServerDeletion")),
    title: t("modals:_trn_confirm_batch_server_deletion")
  },
  serverEulaAcceptance: {
    component: lazy(() => import("./modals/ServerEulaAcceptance")),
    title: t("modals:_trn_server_eula_acceptance")
  },
  confirmBatchFolderDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchFolderDeletion")),
    title: t("modals:_trn_confirm_batch_folder_deletion")
  },
  confirmBatchMixedDeletion: {
    component: lazy(() => import("./modals/ConfirmBatchMixedDeletion")),
    title: t("modals:_trn_confirm_batch_deletion")
  },
  ConfirmChangeRuntimePath: {
    component: lazy(() => import("./modals/ConfirmChangeRuntimePath")),
    title: t("modals:_trn_confirm_change_runtime_path"),
    // Backdrop and side-panel close are blocked while the migration is
    // running. Closing mid-flight would orphan files between old and new
    // runtime paths.
    preventClose: () => isChangingRuntimePath()
  },
  onBoarding: {
    component: lazy(() => import("./modals/OnBoarding")),
    noHeader: true
  },
  whyAreAdsNeeded: {
    component: lazy(() => import("./modals/WhyAreAdsNeeded")),
    title: t("modals:_trn_why_are_ads_needed")
  },
  modsUpdater: {
    component: lazy(() => import("./modals/ModsUpdater")),
    title: t("modals:_trn_mods_updater")
  },
  javaProfileCreation: {
    component: lazy(() => import("./modals/JavaProfileCreationModal")),
    title: t("modals:_trn_create_java_profile")
  },
  windowCloseWarning: {
    component: lazy(() => import("./modals/WindowCloseWarning")),
    title: t("modals:_trn_confirm_quit")
  },
  changelogs: {
    component: lazy(() => import("./modals/Changelogs")),
    title: t("modals:_trn_welcome_new_version")
  },
  confirmGDLAccountDeletion: {
    component: lazy(() => import("./modals/ConfirmGDLAccountDeletion")),
    title: t("modals:_trn_confirm_account_deletion")
  },
  confirmMsWithGDLAccountRemoval: {
    component: lazy(() => import("./modals/ConfirmMsWithGDLAccountRemoval")),
    title: t("modals:_trn_confirm_account_removal")
  },
  accountExpired: {
    component: lazy(() => import("./modals/AccountExpired")),
    title: t("modals:_trn_account_expired")
  },
  changeGDLAccountRecoveryEmail: {
    component: lazy(() => import("./modals/ChangeGDLAccountRecoveryEmail")),
    title: t("modals:_trn_change_recovery_email")
  },
  editGDLProfile: {
    component: lazy(() => import("./modals/EditGDLProfile")),
    title: t("modals:_trn_edit_profile")
  },
  changeGDLAccountDisplayName: {
    component: lazy(() => import("./modals/ChangeGDLAccountDisplayName")),
    title: t("modals:_trn_change_display_name")
  },
  modDetails: {
    component: lazy(() => import("./modals/ModDetails")),
    title: t("modals:_trn_mod_details")
  },
  platformSelection: {
    component: lazy(() => import("./modals/PlatformSelection")),
    title: t("instances:_trn_select_platform")
  },
  duplicatedModsResolution: {
    component: lazy(() => import("./modals/DuplicatedModsResolution")),
    title: t("content:_trn_duplicated_mods_detected")
  },
  betaPrompt: {
    component: lazy(() => import("./modals/BetaPrompt")),
    title: t("modals:_trn_beta_prompt_title")
  },
  shareInstance: {
    component: lazy(() => import("./modals/ShareInstance")),
    title: t("instances:_trn_instance_share.title")
  },
  myShares: {
    component: lazy(() => import("./modals/MyShares")),
    title: t("instances:_trn_my_shares.title")
  },
  editShare: {
    component: lazy(() => import("./modals/EditShare")),
    title: t("instances:_trn_my_shares.edit")
  },
  sharePreview: {
    component: lazy(() => import("./modals/SharePreview")),
    title: t("instances:_trn_share_preview.title")
  },
  report: {
    component: lazy(() => import("./modals/Report")),
    title: t("instances:_trn_report.title")
  },
  accountBanned: {
    component: lazy(() => import("./modals/AccountBanned")),
    preventClose: true,
    noHeader: true
  },
  requiresGdlAccount: {
    component: lazy(() => import("./modals/RequiresGdlAccountModal")),
    title: t("accounts:_trn_requires_gdl_account")
  },
  insufficientMemory: {
    component: lazy(() => import("./modals/InsufficientMemory")),
    title: t("java:_trn_insufficient_memory_title")
  },
  confirmWorldDeletion: {
    component: lazy(() => import("./modals/ConfirmWorldDeletion")),
    title: t("instances:_trn_confirm_world_deletion_title")
  },
  serverCreation: {
    component: lazy(() => import("./modals/ServerCreation")),
    title: "New Server"
  },
  serverRename: {
    component: lazy(() => import("./modals/ServerRename")),
    title: t("modals:_trn_server_rename")
  },
  cacheCleanup: {
    component: lazy(() => import("./modals/CacheCleanup")),
    // Block backdrop/side-panel close only while a cleanup is actively
    // running; closing mid-VACUUM reveals a frozen UI (connection_limit=1
    // serializes every other DB query) and the modal can't be reopened onto
    // the in-flight task. All other phases close normally.
    preventClose: () => cleanupRunning(),
    title: t("modals:_trn_cache_cleanup_title")
  }
})

type ModalName = keyof ReturnType<typeof getDefaultModals>

interface Modal {
  name: ModalName
  url?: string
}

interface Context {
  openModal: (_modal: Modal, _data?: any) => void
  closeModal: () => void
  hasOpenModals: () => boolean
}

export interface StackEntry {
  name: ModalName
  data: any
  /** Live `preventClose` read for this modal instance, set by its mounted
   *  ModalLayout (see `ModalStackEntryContext` below) and cleared on
   *  unmount. Escape/backdrop OR this together with the static registry's
   *  `preventClose` — either source can block the close. */
  preventCloseAccessor?: () => boolean
}

type Stack = StackEntry[]

const ModalsContext = createContext<Context>()

export interface ModalStackEntryApi {
  registerPreventClose: (_accessor: () => boolean) => void
  unregisterPreventClose: () => void
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

  const [_searchParams, setSearchParams] = useSearchParams()

  const closeModal = (name?: ModalName) => {
    setModalStack((currentStack) => {
      const newStack = currentStack.slice()

      // Remove the specific modal or the top modal
      let indexToRemove: number
      if (name) {
        indexToRemove = currentStack.findIndex((modal) => modal.name === name)
      } else {
        indexToRemove = currentStack.length - 1
      }

      if (indexToRemove >= 0) {
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
        setTimeout(() => {
          overlay.style.display = "none"
        }, 100) // Wait for transition to complete
      }
    }
  }

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

    // Escape closes the top modal, mirroring a backdrop click: skipped when the
    // modal opts out of closing (preventClose), and when an open
    // dropdown/select/menu (a Kobalte dismissable layer) should consume the
    // Escape itself so one press doesn't dismiss both.
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented) {
        return
      }
      const stack = modalStack()
      if (stack.length === 0) {
        return
      }
      if (document.querySelector('[role="listbox"],[role="menu"]')) {
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

    onCleanup(() => {
      cleanupMemory()
      cleanupEula()
      document.removeEventListener("keydown", onKeyDown)
    })
  })

  const manager = {
    openModal: (modal: Modal, data: any) => {
      const overlay = document.getElementById("overlay")!
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
    hasOpenModals: () => modalStack().length > 0
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
              const title = (defaultModals as Hash)[modal.name].title || ""

              // Bound to this stack entry's own object (stable for the
              // entry's lifetime — `<For>` only calls this mapper once per
              // item), so the mounted ModalLayout's registration can never
              // land on a different modal instance.
              const stackEntryApi: ModalStackEntryApi = {
                registerPreventClose: (accessor) => {
                  modal.preventCloseAccessor = accessor
                },
                unregisterPreventClose: () => {
                  modal.preventCloseAccessor = undefined
                }
              }

              return (
                <div class="absolute inset-0 flex h-screen w-screen">
                  <div
                    class="z-999 relative flex h-full grow items-center justify-center"
                    onMouseDown={() => {
                      if (!shouldPreventModalClose(modal)) {
                        closeModal()
                      }
                    }}
                  >
                    <div
                      style={{ "z-index": `${index() + 1}` }}
                      onMouseDown={(e) => e.stopPropagation()}
                      class="animate-modalEnter"
                    >
                      <ModalStackEntryContext.Provider value={stackEntryApi}>
                        <Dynamic
                          component={ModalComponent}
                          data={modal.data}
                          noHeader={noHeader}
                          title={title}
                        />
                      </ModalStackEntryContext.Provider>
                    </div>
                    <div class="bg-darkSlate-900 absolute inset-0 opacity-95 transition-opacity duration-100" />
                  </div>

                  <div
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
