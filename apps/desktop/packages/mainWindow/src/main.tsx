/* eslint-disable i18next/no-literal-string */
/* @refresh reload */
import { render } from "solid-js/web";
import { createResource, ErrorBoundary, Show } from "solid-js";
import { Router, hashIntegration } from "@solidjs/router";
import initRspc, { rspc, queryClient } from "@/utils/rspcClient";
import { i18n, TransProvider, icu, loadLanguageFile } from "@gd/i18n";
import App from "@/app";
import { ModalProvider } from "@/managers/ModalsManager";
import initAnalytics from "@/utils/analytics";
import "virtual:uno.css";
import "@gd/ui/style.css";
import { NotificationsProvider } from "@gd/ui";
import { NavigationManager } from "./managers/NavigationManager";

queueMicrotask(() => {
  initAnalytics();
});

render(() => {
  let allowedToClear = false;
  const [coreModuleLoaded] = createResource(async () => {
    let port = await window.getCoreModuleStatus();
    if (allowedToClear) window.clearLoading();
    return port;
  });

  setTimeout(() => {
    if (coreModuleLoaded() as unknown as number) {
      window.clearLoading();
    } else {
      allowedToClear = true;
    }
  }, 500);

  const [i18nInstance] = createResource(async () => {
    const DEFAULT_LANG = "en";
    const langFile = await loadLanguageFile(DEFAULT_LANG);

    const instance = await i18n.use(icu).createInstance({
      defaultNS: "common",
      fallbackLng: DEFAULT_LANG,
      resources: {
        [DEFAULT_LANG]: {
          common: langFile,
        },
      },
    });

    return instance;
  });

  return (
    <Show
      when={
        i18nInstance.state === "ready" && coreModuleLoaded.state === "ready"
      }
    >
      <InnerApp
        port={coreModuleLoaded() as unknown as number}
        i18nInstance={i18nInstance() as unknown as typeof i18n}
      />
    </Show>
  );
}, document.getElementById("root") as HTMLElement);

type InnerAppProps = {
  port: number;
  i18nInstance: typeof i18n;
};

const InnerApp = (props: InnerAppProps) => {
  // eslint-disable-next-line solid/reactivity
  let { client, createInvalidateQuery } = initRspc(props.port);

  return (
    <ErrorBoundary
      fallback={(err) => {
        console.log("ERR", err);
        return (
          <div class="relative w-screen h-screen z-100 flex justify-center items-center">
            <div class="flex flex-col">
              <h1 class="m-0">Error</h1>
              <p>{err.message}</p>
              <p>{err.stack}</p>
              {err.message}
            </div>
          </div>
        );
      }}
    >
      <rspc.Provider client={client as any} queryClient={queryClient}>
        <Router source={hashIntegration()}>
          <NavigationManager>
            <TransProvider instance={props.i18nInstance}>
              <NotificationsProvider>
                <ModalProvider>
                  <App createInvalidateQuery={createInvalidateQuery} />
                </ModalProvider>
              </NotificationsProvider>
            </TransProvider>
          </NavigationManager>
        </Router>
      </rspc.Provider>
    </ErrorBoundary>
  );
};
