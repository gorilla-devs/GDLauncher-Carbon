/* @refresh reload */
import { render } from "solid-js/web"
import {
  createContext,
  createEffect,
  createResource,
  createSignal,
  ErrorBoundary,
  JSX,
  Match,
  Show,
  Switch,
  useContext
} from "solid-js"
import { createAsyncEffect } from "@/utils/asyncEffect"
import { Router, hashIntegration } from "@solidjs/router"
import initRspc, { rspc, queryClient } from "@/utils/rspcClient"
import { i18n, TransProvider, loadLanguageFiles } from "@gd/i18n"
import App from "@/app"
import { ModalProvider } from "@/managers/ModalsManager"
import "virtual:uno.css"
import "@gd/ui/style.css"
import { ContextMenuProvider, Toaster } from "@gd/ui"
import "@unocss/reset/tailwind.css"
import { NavigationManager } from "./managers/NavigationManager"
import RiveAppWapper from "./utils/RiveAppWrapper"
import GDAnimation from "./gd_logo_animation.riv"
import { GlobalStoreProvider } from "./components/GlobalStoreContext"
import AuthLoadingOverlay from "./pages/Login/AuthLoadingOverlay"
import { OnboardingProvider, SpotlightOverlay } from "./components/Onboarding"

const BackendReadyContext = createContext<boolean>(false)

export const useBackendReady = () => {
  const context = useContext(BackendReadyContext)
  return context
}

const ProdWrapErrorBoundary = (props: { children: JSX.Element }) => {
  return (
    <Switch>
      <Match when={!import.meta.env.DEV}>
        <ErrorBoundary
          fallback={(err) => {
            console.error("Window errored", err)
            window.fatalError(err, "Window")
            return <></>
          }}
        >
          {props.children}
        </ErrorBoundary>
      </Match>
      <Match when={import.meta.env.DEV}>{props.children}</Match>
    </Switch>
  )
}

render(() => {
  const [coreModuleProgress, setCoreModuleProgress] = createSignal<
    number | undefined
  >(undefined)

  const [coreModuleLoaded] = createResource(async () => {
    let port
    try {
      const coreModule = await window.getCoreModule()

      if (coreModule?.type === "success") {
        const convertedPort = Number(coreModule.port)
        port = convertedPort
      } else if (coreModule?.type === "backwardsMigration") {
        console.log("Backwards migration detected, showing dedicated UI")
        window.backwardsMigrationError()
        port = new Error("BackwardsMigration")
      } else {
        if (coreModule.logs) {
          console.error(
            "CoreModule errored",
            JSON.stringify(coreModule, null, 2)
          )
          window.fatalError(coreModule.logs, "CoreModule")
        } else {
          console.error("CoreModule errored with no logs", coreModule)
          window.fatalError("Unknown error", "CoreModule")
        }

        port = new Error("CoreModule")
      }
    } catch (e) {
      console.error("CoreModule getCoreModule failed", e)
      window.fatalError(e as any, "CoreModule")
      port = new Error("CoreModule")
    }

    if (port instanceof Error) {
      throw port
    }

    return port
  })

  window.listenToCoreModuleProgress((_, progress) => {
    setCoreModuleProgress(progress)
  })

  // Request any buffered progress that was sent before listener was ready
  window.getCurrentProgress().then((progress) => {
    if (progress !== null && progress > (coreModuleProgress() ?? 0)) {
      setCoreModuleProgress(progress)
    }
  })

  const startTime = Date.now()

  const [isReady, setIsReady] = createSignal(false)
  const [isIntroAnimationFinished, setIsIntroAnimationFinished] = createSignal(
    window.skipIntroAnimation
  )

  createEffect(() => {
    if (!isIntroAnimationFinished()) return

    const minLoadingTime = 3000
    const timeElapsed = Date.now() - startTime

    if (coreModuleLoaded.state === "ready" && timeElapsed >= minLoadingTime) {
      setIsReady(true)
    } else if (coreModuleLoaded.state === "ready") {
      setTimeout(() => {
        setIsReady(true)
      }, minLoadingTime - timeElapsed)
    }
  })

  return (
    <ProdWrapErrorBoundary>
      <Switch>
        <Match when={isIntroAnimationFinished()}>
          <Switch>
            <Match when={isReady()}>
              <InnerApp
                port={coreModuleLoaded() as unknown as number}
                isBackendReady={true}
              />
              <Toaster />
            </Match>
            <Match when={!isReady()}>
              <AuthLoadingOverlay
                progress={coreModuleProgress() ?? 0}
                status={null}
                visible={true}
              />
            </Match>
          </Switch>
        </Match>
        <Match when={!isIntroAnimationFinished()}>
          <div class="flex h-screen w-full items-center justify-center">
            <RiveAppWapper
              src={GDAnimation}
              onStop={() => {
                setIsIntroAnimationFinished(true)
              }}
            />
          </div>
        </Match>
      </Switch>
    </ProdWrapErrorBoundary>
  )
}, document.getElementById("root")!)

interface InnerAppProps {
  port: number
  isBackendReady: boolean
}

const InnerApp = (props: InnerAppProps) => {
  const { client, createInvalidateQuery } = initRspc(props.port)

  return (
    <rspc.Provider client={client} queryClient={queryClient}>
      <TransWrapper
        createInvalidateQuery={createInvalidateQuery}
        isBackendReady={props.isBackendReady}
      />
    </rspc.Provider>
  )
}

interface TransWrapperProps {
  createInvalidateQuery: () => void
  isBackendReady: boolean
}

const _i18nInstance = i18n

const TransWrapper = (props: TransWrapperProps) => {
  const [isI18nReady, setIsI18nReady] = createSignal(false)
  const [i18nOptions, setI18nOptions] = createSignal<any>(null)

  const trackPageView = rspc.createMutation(() => ({
    mutationKey: "metrics.sendEvent"
  }))

  window.addEventListener("hashchange", () => {
    trackPageView.mutate({
      event_name: "page_view",
      data: window.location.hash
    })
  })

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  createAsyncEffect((isStale, prevLanguage) => {
    if (settings.isSuccess) {
      const { language } = settings.data

      loadLanguageFiles("english").then((englishResources) => {
        if (isStale()) {
          return
        }

        const resources: any = { english: englishResources }

        if (language !== "english") {
          loadLanguageFiles(language).then((targetResources) => {
            if (isStale()) {
              return
            }

            resources[language] = targetResources

            const options = {
              ns: Object.keys(englishResources),
              defaultNS: "general",
              lng: language,
              fallbackLng: "english",
              resources,
              partialBundledLanguages: true,
              react: { useSuspense: false }
            }

            setI18nOptions(options)
            setIsI18nReady(true)
          })
        } else {
          const options = {
            ns: Object.keys(englishResources),
            defaultNS: "general",
            lng: language,
            fallbackLng: "english",
            resources,
            partialBundledLanguages: true,
            react: { useSuspense: false }
          }

          setI18nOptions(options)
          setIsI18nReady(true)
        }
      })
    }

    return prevLanguage
  }, undefined)

  createEffect(() => {
    const root = document.getElementById("root")
    const overlay = document.getElementById("overlay")
    if (root && overlay) {
      if (settings.data?.reducedMotion) {
        root.classList.add("potato-pc")
        overlay.classList.add("potato-pc")
      } else {
        root.classList.remove("potato-pc")
        overlay.classList.remove("potato-pc")
      }
    }
  })

  return (
    <Show when={!settings.isInitialLoading && isI18nReady() && i18nOptions()}>
      <TransProvider instance={_i18nInstance} options={i18nOptions()}>
        <BackendReadyContext.Provider value={props.isBackendReady}>
          <Router source={hashIntegration()}>
            <GlobalStoreProvider>
              <OnboardingProvider>
                <NavigationManager>
                  <ContextMenuProvider>
                    <ModalProvider>
                      <App
                        createInvalidateQuery={props.createInvalidateQuery}
                      />
                      <SpotlightOverlay />
                    </ModalProvider>
                  </ContextMenuProvider>
                </NavigationManager>
              </OnboardingProvider>
            </GlobalStoreProvider>
          </Router>
        </BackendReadyContext.Provider>
      </TransProvider>
    </Show>
  )
}
