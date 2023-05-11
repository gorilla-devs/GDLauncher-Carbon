/* @refresh reload */
import { render } from "solid-js/web";
import { createContext, createResource, Show, useContext } from "solid-js";
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

const ClientRspcContext = createContext();

export function useClientRspc() {
  return useContext(ClientRspcContext);
}

const InnerApp = (props: InnerAppProps) => {
  // eslint-disable-next-line solid/reactivity
  let { client, createInvalidateQuery } = initRspc(props.port);

  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <ClientRspcContext.Provider value={client}>
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
      </ClientRspcContext.Provider>
    </rspc.Provider>
  );
};
