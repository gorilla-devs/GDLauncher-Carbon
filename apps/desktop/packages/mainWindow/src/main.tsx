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
import { i18n, TransProvider, icu, loadLanguageFiles } from "@gd/i18n"
import App from "@/app"
import { ModalProvider } from "@/managers/ModalsManager"
import "virtual:uno.css"
import "@gd/ui/style.css"
import { ContextMenuProvider, Toaster } from "@gd/ui"
import "@unocss/reset/tailwind.css"
import { NavigationManager } from "./managers/NavigationManager"
// import { ContextMenuProvider } from "./components/ContextMenu/ContextMenuContext";
import RiveAppWapper from "./utils/RiveAppWrapper"
import GDAnimation from "./gd_logo_animation.riv"
import { GlobalStoreProvider } from "./components/GlobalStoreContext"
import AuthLoadingOverlay from "./pages/Login/AuthLoadingOverlay"
import { isSpecialOccasion } from "@/utils/occasions"

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
    const startProgress = coreModuleProgress() ?? 0
    const endProgress = progress
    const duration = 300
    const startTime = Date.now()

    const easeOutCubic = (x: number): number => {
      return 1 - Math.pow(1 - x, 3)
    }

    const animate = () => {
      const currentTime = Date.now()
      const elapsed = currentTime - startTime
      const progress = Math.min(elapsed / duration, 1)

      if (progress < 1) {
        const easedProgress = easeOutCubic(progress)
        const currentValue =
          startProgress + (endProgress - startProgress) * easedProgress
        setCoreModuleProgress(currentValue)
        requestAnimationFrame(animate)
      } else {
        setCoreModuleProgress(endProgress)
      }
    }

    requestAnimationFrame(animate)
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

  // Smart auth bypass: Skip login screen for logged-in users (unless special occasion)
  createAsyncEffect(async () => {
    if (coreModuleLoaded.state === "ready" && coreModuleLoaded()) {
      const port = coreModuleLoaded() as unknown as number
      const { client } = initRspc(port)

      try {
        const settings = await client.query(["settings.getSettings"])
        const activeUuid = await client.query(["account.getActiveUuid"])
        const accounts = await client.query(["account.getAccounts"])

        // Check if it's a special occasion
        const isOccasion = isSpecialOccasion()

        // Skip directly to library if:
        // 1. User is fully set up (has accounts, terms accepted, made GDL account decision)
        // 2. AND it's NOT a special occasion (we want to show seasonal splash during occasions)
        if (
          settings.termsAndPrivacyAccepted &&
          activeUuid &&
          accounts.length > 0 &&
          settings.gdlAccountId != null &&
          !isOccasion
        ) {
          console.log(
            "[Auth Bypass] Skipping to library (logged in, no occasion)"
          )
          window.location.hash = "#/library"
        } else if (accounts.length > 0 && isOccasion) {
          console.log(
            "[Auth Bypass] Will show seasonal splash (logged in + occasion)"
          )
          // Let InnerApp/LoginContainer handle seasonal splash
        }
      } catch (e) {
        console.error("Error checking login status:", e)
      }
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

const _i18nInstance = i18n.use(icu).createInstance()

const TransWrapper = (props: TransWrapperProps) => {
  const [isI18nReady, setIsI18nReady] = createSignal(false)
  // const rspcContext = rspc.useContext();

  // onMount(async () => {
  //   while (true) {
  //     let initialTime = Date.now();

  //     await rspcContext.client.query(["echo", "something"]);

  //     let elapsed = Date.now() - initialTime;

  //     console.log("rspc latency (ms)", elapsed);

  //     await new Promise((resolve) => setTimeout(resolve, 200));
  //   }
  // });

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

  createAsyncEffect<string>((isStale, prevLanguage) => {
    if (settings.isSuccess) {
      const { language } = settings.data

      if (!_i18nInstance.isInitialized) {
        const currentLanguage = language

        const initI18n = async () => {
          let maybeEnglish = null
          if (currentLanguage !== "english") {
            maybeEnglish = await loadLanguageFiles("english")
          }

          // Check if language hasn't changed during async load
          if (isStale()) {
            return
          }

          const defaultNamespacesMap = await loadLanguageFiles(currentLanguage)

          // Check again after second async load
          if (isStale()) {
            return
          }

          await _i18nInstance.init({
            ns: Object.keys(defaultNamespacesMap),
            defaultNS: "common",
            lng: currentLanguage,
            fallbackLng: "english",
            resources: {
              [currentLanguage]: defaultNamespacesMap,
              ...(maybeEnglish && { english: maybeEnglish })
            },
            partialBundledLanguages: true,
            debug: true
          })

          setIsI18nReady(true)
        }

        initI18n()
      }

      return language
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
    <Show when={!settings.isInitialLoading && isI18nReady()}>
      <TransProvider instance={_i18nInstance}>
        <BackendReadyContext.Provider value={props.isBackendReady}>
          <Router source={hashIntegration()}>
            <GlobalStoreProvider>
              <NavigationManager>
                <ContextMenuProvider>
                  <ModalProvider>
                    <App createInvalidateQuery={props.createInvalidateQuery} />
                  </ModalProvider>
                </ContextMenuProvider>
              </NavigationManager>
            </GlobalStoreProvider>
          </Router>
        </BackendReadyContext.Provider>
      </TransProvider>
    </Show>
  )
}
