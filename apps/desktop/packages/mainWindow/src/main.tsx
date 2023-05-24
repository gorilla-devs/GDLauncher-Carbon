/* @refresh reload */
import { render } from "solid-js/web";
import { createResource, Show } from "solid-js";
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
import { DEFAULT_LANG, LANGUAGES } from "./constants";
import { ContextMenuProvider } from "./components/ContextMenu/ContextMenuContext";

queueMicrotask(() => {
  initAnalytics();
});

interface Translations {
  [translationKey: string]: string;
}

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
    const buildResources = async (langs: string[]) => {
      const langFilesPromises = langs.map(loadLanguageFile);
      const langFiles: Translations = (await Promise.all(
        langFilesPromises
      )) as any;

      const resources: { [translationKey: string]: Translations } = {};
      for (let i = 0; i < langs.length; i++) {
        resources[langs[i]] = {
          common: langFiles[i],
        };
      }

      return resources;
    };

    const resources = await buildResources(LANGUAGES);

    const instance = await i18n.use(icu).createInstance({
      defaultNS: "common",
      fallbackLng: DEFAULT_LANG,
      resources: resources,
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
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <NavigationManager>
          <TransProvider instance={props.i18nInstance} options={{ lng: "en" }}>
            <NotificationsProvider>
              <ContextMenuProvider>
                <ModalProvider>
                  <App createInvalidateQuery={createInvalidateQuery} />
                </ModalProvider>
              </ContextMenuProvider>
            </NotificationsProvider>
          </TransProvider>
        </NavigationManager>
      </Router>
    </rspc.Provider>
  );
};
