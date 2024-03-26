/* @refresh reload */
import { render } from "solid-js/web";
import {
  createEffect,
  createResource,
  createSignal,
  ErrorBoundary,
  Match,
  Show,
  Switch
} from "solid-js";
import { Router, hashIntegration } from "@solidjs/router";
import initRspc, { rspc, queryClient } from "@/utils/rspcClient";
import { i18n, TransProvider, icu, loadLanguageFiles } from "@gd/i18n";
import App from "@/app";
import { ModalProvider } from "@/managers/ModalsManager";
import "virtual:uno.css";
import "@gd/ui/style.css";
import { ContextMenuProvider, NotificationsProvider } from "@gd/ui";
import { NavigationManager } from "./managers/NavigationManager";
// import { ContextMenuProvider } from "./components/ContextMenu/ContextMenuContext";
import RiveAppWapper from "./utils/RiveAppWrapper";
import GDAnimation from "./gd_logo_animation.riv";

render(
  () => {
    const [coreModuleLoaded] = createResource(async () => {
      let port;
      try {
        const coreModule = await window.getCoreModule();

        if (coreModule?.type === "success") {
          const convertedPort = Number(coreModule.port);
          port = convertedPort;
        } else {
          if (coreModule.logs) {
            console.error("CoreModule errored", coreModule);
            window.fatalError(coreModule.logs, "CoreModule");
          } else {
            console.error("CoreModule errored with no logs", coreModule);
            window.fatalError("Unknown error", "CoreModule");
          }

          port = new Error("CoreModule");
        }
      } catch (e) {
        console.error("CoreModule getCoreModule failed", e);
        window.fatalError(e as any, "CoreModule");
        port = new Error("CoreModule");
      }

      if (port instanceof Error) {
        throw port;
      }

      return port;
    });

    const [isReady, setIsReady] = createSignal(false);
    const [isIntroAnimationFinished, setIsIntroAnimationFinished] =
      createSignal(window.skipIntroAnimation);

    createEffect(() => {
      if (!isIntroAnimationFinished()) return;

      setIsReady(coreModuleLoaded.state === "ready");
    });

    return (
      <ErrorBoundary
        fallback={(err) => {
          console.error("Window errored", err);

          window.fatalError(err, "Window");

          return <></>;
        }}
      >
        <Switch>
          <Match when={isIntroAnimationFinished()}>
            <Switch>
              <Match when={isReady()}>
                <NotificationsProvider>
                  <InnerApp port={coreModuleLoaded() as unknown as number} />
                </NotificationsProvider>
              </Match>
              <Match when={!isReady()}>
                <div class="flex justify-center items-center h-screen w-screen">
                  <div class="animate-spin rounded-full h-12 w-12 bg-blue-500 i-ri:loader-4-line" />
                </div>
              </Match>
            </Switch>
          </Match>
          <Match when={!isIntroAnimationFinished()}>
            <div class="w-full flex justify-center items-center h-screen">
              <RiveAppWapper
                src={GDAnimation}
                onStop={() => {
                  setIsIntroAnimationFinished(true);
                }}
              />
            </div>
          </Match>
        </Switch>
      </ErrorBoundary>
    );
  },
  document.getElementById("root") as HTMLElement
);

type InnerAppProps = {
  port: number;
};

const InnerApp = (props: InnerAppProps) => {
  // eslint-disable-next-line solid/reactivity
  let { client, createInvalidateQuery } = initRspc(props.port);

  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <TransWrapper createInvalidateQuery={createInvalidateQuery} />
    </rspc.Provider>
  );
};

type TransWrapperProps = {
  createInvalidateQuery: () => void;
};

const _i18nInstance = i18n.use(icu).createInstance();

const TransWrapper = (props: TransWrapperProps) => {
  const [isI18nReady, setIsI18nReady] = createSignal(false);
  const trackPageView = rspc.createMutation(() => ({
    mutationKey: "metrics.sendEvent"
  }));

  window.addEventListener("hashchange", () => {
    trackPageView.mutate({
      event_name: "page_view",
      data: window.location.hash
    });
  });

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  createEffect(async () => {
    if (settings.isSuccess) {
      let { language } = settings.data;
      if (!_i18nInstance.isInitialized) {
        let maybeEnglish = null;
        if (language !== "english") {
          maybeEnglish = await loadLanguageFiles("english");
        }

        const defaultNamespacesMap = await loadLanguageFiles(language);

        await _i18nInstance.init({
          ns: Object.keys(defaultNamespacesMap),
          defaultNS: "common",
          lng: language,
          fallbackLng: "english",
          resources: {
            [language]: defaultNamespacesMap,
            ...(maybeEnglish && { english: maybeEnglish })
          },
          partialBundledLanguages: true,
          debug: true
        });

        setIsI18nReady(true);

        return;
      }
    }
  });

  createEffect(() => {
    const root = document.getElementById("root");
    const overlay = document.getElementById("overlay");
    if (root && overlay) {
      if (settings.data?.reducedMotion) {
        root.classList.add("potato-pc");
        overlay.classList.add("potato-pc");
      } else {
        root.classList.remove("potato-pc");
        overlay.classList.remove("potato-pc");
      }
    }
  });

  return (
    <Show when={!settings.isInitialLoading && isI18nReady()}>
      <TransProvider instance={_i18nInstance}>
        <Router source={hashIntegration()}>
          <NavigationManager>
            <ContextMenuProvider>
              <ModalProvider>
                <App createInvalidateQuery={props.createInvalidateQuery} />
              </ModalProvider>
            </ContextMenuProvider>
          </NavigationManager>
        </Router>
      </TransProvider>
    </Show>
  );
};
