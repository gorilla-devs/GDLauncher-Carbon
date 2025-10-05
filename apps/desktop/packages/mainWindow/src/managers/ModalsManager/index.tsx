import { useLocation, useSearchParams } from "@solidjs/router"
import {
  createContext,
  createSignal,
  For,
  JSX,
  lazy,
  useContext
} from "solid-js"
import { Dynamic, Portal } from "solid-js/web"
import { useTransContext } from "@gd/i18n"
import { useGDNavigate } from "../NavigationManager"
import adSize from "@/utils/adhelper"

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
    preventClose?: boolean
    title?: string
    noHeader?: boolean
  }
>

const getDefaultModals = (t: (key: string) => string) => ({
  privacyStatement: {
    component: lazy(() => import("./modals/PrivacyStatement")),
    title: t("modals.privacy_statement")
  },
  termsAndConditions: {
    component: lazy(() => import("./modals/TermsAndConditions")),
    title: t("modals.terms_and_conditions")
  },
  addManagedJava: {
    component: lazy(() => import("./modals/Java/AddManagedJava")),
    title: t("modals.add_java_version")
  },
  addCustomJava: {
    component: lazy(() => import("./modals/Java/AddCustomJava")),
    title: t("modals.add_java_version")
  },
  javaSetup: {
    component: lazy(() => import("./modals/Java/JavaSetup")),
    title: t("modals.java_setup")
  },
  instanceCreation: {
    component: lazy(() => import("./modals/InstanceCreation")),
    title: t("modals.new_instance")
  },
  exportInstance: {
    component: lazy(() => import("./modals/InstanceExport")),
    title: t("modals.export_instance")
  },
  modpack_version_update: {
    component: lazy(() => import("./modals/ModPackVersionUpdate")),
    title: t("modals.change_modpack_version")
  },
  unlock_confirmation: {
    component: lazy(() => import("./modals/Confirmation")),
    title: t("modals.unlock_instance")
  },
  unpair_confirmation: {
    component: lazy(() => import("./modals/Confirmation")),
    title: t("modals.unpair_instance")
  },
  notification: {
    component: lazy(() => import("./modals/Notification")),
    title: t("modals.notification")
  },
  confirmInstanceDeletion: {
    component: lazy(() => import("./modals/ConfirmInstanceDeletion")),
    title: t("modals.confirm_instance_deletion")
  },
  ConfirmChangeRuntimePath: {
    component: lazy(() => import("./modals/ConfirmChangeRuntimePath")),
    title: t("modals.confirm_change_runtime_path")
  },
  appUpdate: {
    component: lazy(() => import("./modals/AppUpdate")),
    title: t("modals.new_app_version_available")
  },
  onBoarding: {
    component: lazy(() => import("./modals/OnBoarding")),
    noHeader: true
  },
  whyAreAdsNeeded: {
    component: lazy(() => import("./modals/WhyAreAdsNeeded")),
    title: t("modals.why_are_ads_needed")
  },
  modsUpdater: {
    component: lazy(() => import("./modals/ModsUpdater")),
    title: t("modals.mods_updater")
  },
  javaProfileCreation: {
    component: lazy(() => import("./modals/JavaProfileCreationModal")),
    title: t("modals.create_java_profile")
  },
  windowCloseWarning: {
    component: lazy(() => import("./modals/WindowCloseWarning")),
    title: t("modals.confirm_quit")
  },
  changelogs: {
    component: lazy(() => import("./modals/Changelogs")),
    title: t("modals.welcome_new_version")
  },
  confirmGDLAccountDeletion: {
    component: lazy(() => import("./modals/ConfirmGDLAccountDeletion")),
    title: t("modals.confirm_account_deletion")
  },
  confirmMsWithGDLAccountRemoval: {
    component: lazy(() => import("./modals/ConfirmMsWithGDLAccountRemoval")),
    title: t("modals.confirm_account_removal")
  },
  accountExpired: {
    component: lazy(() => import("./modals/AccountExpired")),
    title: t("modals.account_expired")
  },
  bisectHostingAffiliate: {
    component: lazy(() => import("./modals/BisectHostingAffiliate")),
    title: t("modals.bisecthosting_affiliate")
  },
  changeGDLAccountRecoveryEmail: {
    component: lazy(() => import("./modals/ChangeGDLAccountRecoveryEmail")),
    title: t("modals.change_recovery_email")
  },
  modDetails: {
    component: lazy(() => import("./modals/ModDetails")),
    title: t("modals.mod_details")
  },
  platformSelection: {
    component: lazy(() => import("./modals/PlatformSelection")),
    title: t("instance.select_platform")
  },
  confirmCacheClear: {
    component: lazy(() => import("./modals/ConfirmCacheClear")),
    title: t("modals.confirm_cache_clear")
  },
  duplicatedModsResolution: {
    component: lazy(() => import("./modals/DuplicatedModsResolution")),
    title: t("Fix Duplicated Mods")
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
}

type Stack = { name: ModalName; data: any }[]

const ModalsContext = createContext<Context>()

export const ModalProvider = (props: { children: JSX.Element }) => {
  const [t] = useTransContext()
  const defaultModals = getDefaultModals(t)
  const navigator = useGDNavigate()
  const location = useLocation()
  const queryParams = () => location.search as ModalName
  const urlSearchParams = () => new URLSearchParams(queryParams())
  const [modalStack, setModalStack] = createSignal<Stack>([])

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
        const newParams: Record<string, string | null> =
          Object.fromEntries(urlSearchParams())

        for (const key in Object.fromEntries(urlSearchParams())) {
          if (key !== `m[${indexToRemove + 1}]`) {
            newParams[`m[${indexToRemove + 1}]`] = null
          }
        }

        setSearchParams(newParams)
      }

      return newStack
    })

    if (modalStack().length === 0) {
      const overlay = document.getElementById("overlay")!
      overlay.style.display = "none"
    }
  }

  const manager = {
    openModal: (modal: Modal, data: any) => {
      const overlay = document.getElementById("overlay")!
      overlay.style.display = "flex"
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
    closeModal
  }

  return (
    <ModalsContext.Provider value={manager}>
      {props.children}
      <Portal mount={document.getElementById("overlay")!}>
        <div class="h-screen w-screen">
          <For each={modalStack()}>
            {(modal, index) => {
              const ModalComponent = defaultModals[modal.name].component
              const noHeader =
                (defaultModals as Hash)[modal.name].noHeader || false
              const title = (defaultModals as Hash)[modal.name].title || ""
              const preventClose = (defaultModals as Hash)[modal.name]
                .preventClose

              return (
                <div class="absolute inset-0 flex h-screen w-screen">
                  <div
                    class="z-999 relative flex h-full flex-grow items-center justify-center"
                    onMouseDown={() => {
                      if (!preventClose) {
                        closeModal()
                      }
                    }}
                  >
                    <div
                      style={{ "z-index": `${index() + 1}` }}
                      onMouseDown={(e) => e.stopPropagation()}
                      class="animate-enterWithOpacityChange duration-100 ease-in-out"
                    >
                      <Dynamic
                        component={ModalComponent}
                        data={modal.data}
                        noHeader={noHeader}
                        title={title}
                      />
                    </div>
                    <div class="bg-darkSlate-900 absolute inset-0 opacity-80" />
                  </div>

                  <div
                    class="text-lightSlate-50 z-999 bg-darkSlate-900 h-screen origin-center place-items-center opacity-80 duration-100 ease-in-out"
                    style={{
                      width: `${adSize.width}px`
                    }}
                    onMouseDown={() => {
                      if (!preventClose) {
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
