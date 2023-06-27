/* @refresh reload */
import { render } from "solid-js/web";
import {
  createEffect,
  createResource,
  createSignal,
  Match,
  Show,
  Switch,
} from "solid-js";
import { Router, hashIntegration } from "@solidjs/router";
import initRspc, { rspc, queryClient } from "@/utils/rspcClient";
import {
  i18n,
  TransProvider,
  icu,
  loadLanguageFile,
  supportedLanguages,
  loadLanguagesFile,
} from "@gd/i18n";
import App from "@/app";
import { ModalProvider } from "@/managers/ModalsManager";
import "virtual:uno.css";
import "@gd/ui/style.css";
import { NotificationsProvider } from "@gd/ui";
import { NavigationManager } from "./managers/NavigationManager";
import { ContextMenuProvider } from "./components/ContextMenu/ContextMenuContext";
import RiveAppWapper from "./utils/RiveAppWapper";
import GDAnimation from "./gd_logo_animation.riv";

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
}, document.getElementById("root") as HTMLElement);

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

const loadLanguageResources = async (lang: string) => {
  let langFile = await loadLanguageFile(lang);
  let languagesFile = await loadLanguagesFile();

  return {
    common: langFile,
    languages: languagesFile,
  };
};

const _i18nInstance = i18n.use(icu).createInstance();

const TransWrapper = (props: TransWrapperProps) => {
  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newSettings) => {
      queryClient.setQueryData(["settings.getSettings"], newSettings);
    },
  });

  const settings = rspc.createQuery(() => ["settings.getSettings"], {
    async onSuccess(settings) {
      let { language } = settings;
      if (!_i18nInstance.isInitialized) {
        await _i18nInstance.init({
          ns: ["common", "languages"],
          defaultNS: "common",
          lng: language,
          fallbackLng: "english",
          resources: {
            [language]: (await loadLanguageResources(language)) as any,
          },
          partialBundledLanguages: true,
          // debug: true,
        });
      } else {
        if (language === _i18nInstance.language) {
          return;
        }
      }

      if (!Object.keys(supportedLanguages).includes(language)) {
        console.warn(`Language ${language} is not supported`);
        return;
      }

      const previousLanguage = _i18nInstance.language;

      const resources = await loadLanguageResources(language);

      if (!resources.common || !resources.languages) {
        if (previousLanguage) {
          settingsMutation.mutate({
            language: previousLanguage,
          });
        }
        return;
      }

      if (!_i18nInstance.hasResourceBundle(language, "common")) {
        _i18nInstance.addResourceBundle(language, "common", resources.common);
      }
      if (!_i18nInstance.hasResourceBundle(language, "languages")) {
        _i18nInstance.addResourceBundle(
          language,
          "languages",
          resources.languages
        );
      }

      _i18nInstance.changeLanguage(language);

      if (
        previousLanguage &&
        language !== previousLanguage &&
        previousLanguage !== "english"
      ) {
        if (_i18nInstance.hasResourceBundle(previousLanguage, "common")) {
          _i18nInstance.removeResourceBundle(previousLanguage, "common");
        }
        if (_i18nInstance.hasResourceBundle(previousLanguage, "languages")) {
          _i18nInstance.removeResourceBundle(previousLanguage, "languages");
        }
      }

      _i18nInstance.setDefaultNamespace("common");
    },
  });

  return (
    <Show when={!settings.isInitialLoading}>
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
