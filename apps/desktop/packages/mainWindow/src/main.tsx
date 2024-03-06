/* @refresh reload */
import { render } from "solid-js/web";
import {
  createEffect,
  createResource,
  createSignal,
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
        port = await window.getCoreModulePort();

        const convertedPort = Number(port);

        if (Number.isNaN(convertedPort)) {
          console.error("CoreModule port is not a number", port);
          window.fatalError(port as any, "CoreModule");
          port = new Error("CoreModule");
        } else {
          port = convertedPort;
        }
      } catch (e) {
        console.error("CoreModule port failed", e);
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
      <Switch>
        <Match when={isReady()}>
          <InnerApp port={coreModuleLoaded() as unknown as number} />
        </Match>
        <Match when={!isReady()}>
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
  const trackPageView = rspc.createMutation(["metrics.sendEvent"]);

  window.addEventListener("hashchange", () => {
    trackPageView.mutate({
      event_name: "page_view",
      data: window.location.hash
    });
  });

  const settings = rspc.createQuery(() => ["settings.getSettings"], {
    async onSuccess(settings) {
      let { language } = settings;
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
            <NotificationsProvider>
              <ContextMenuProvider>
                <ModalProvider>
                  <App createInvalidateQuery={props.createInvalidateQuery} />
                </ModalProvider>
              </ContextMenuProvider>
            </NotificationsProvider>
          </NavigationManager>
        </Router>
      </TransProvider>
    </Show>
  );
};
