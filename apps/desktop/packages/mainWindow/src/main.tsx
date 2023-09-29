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
import { NotificationsProvider } from "@gd/ui";
import { NavigationManager } from "./managers/NavigationManager";
import { ContextMenuProvider } from "./components/ContextMenu/ContextMenuContext";
import RiveAppWapper from "./utils/RiveAppWrapper";
import GDAnimation from "./gd_logo_animation.riv";
import "@/utils/analytics"; // preinit

render(
  () => {
    const [coreModuleLoaded] = createResource(async () => {
      let port;
      try {
        port = await window.getCoreModulePort();
      } catch (e) {
        window.fatalError(e as string, "CoreModule");
        throw e;
      }
      return port;
    });

    const [isReady, setIsReady] = createSignal(false);

    createEffect(() => {
      if (process.env.NODE_ENV === "development") {
        setIsReady(coreModuleLoaded.state === "ready");
      }
    });

    return (
      <Switch
        fallback={
          <div class="w-full flex justify-center items-center h-screen">
            <RiveAppWapper
              src={GDAnimation}
              onStop={() => {
                setIsReady(coreModuleLoaded.state === "ready");
              }}
            />
          </div>
        }
      >
        <Match when={isReady()}>
          <InnerApp port={coreModuleLoaded() as unknown as number} />
        </Match>
        <Match when={!isReady() && process.env.NODE_ENV !== "development"}>
          <div class="w-full flex justify-center items-center h-screen">
            <RiveAppWapper
              src={GDAnimation}
              onStop={() => {
                setIsReady(coreModuleLoaded.state === "ready");
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

  const settings = rspc.createQuery(() => ["settings.getSettings"], {
    async onSuccess(settings) {
      let { language } = settings;
      if (!_i18nInstance.isInitialized) {
        const defaultNamespacesMap = await loadLanguageFiles(language);

        await _i18nInstance.init({
          ns: Object.keys(defaultNamespacesMap),
          defaultNS: "common",
          lng: language,
          fallbackLng: "english",
          resources: {
            [language]: defaultNamespacesMap
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
