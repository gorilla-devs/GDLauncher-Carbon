/* @refresh reload */
import { onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import { client, queryClient, rspc } from "@/utils/rspcClient";
import App from "@/app";
import Modals from "@/ModalsManager";
import initAnalytics from "@/utils/analytics";
import { initModules } from "@/modules";
import "virtual:uno.css";
import "virtual:unocss-devtools";
import "@gd/ui/style.css";
import { createI18nContext, I18nContext } from "@solid-primitives/i18n";
import { createStore } from "solid-js/store";
import { getTranslationByLanguage } from "@gd/i18n";
interface Translations {
  [key: string]: string;
}
interface LanguagesHashMap {
  [key: string]: Translations;
}

queueMicrotask(() => {
  initAnalytics();
});

render(() => {
  const [languages, setLanguages] = createStore<LanguagesHashMap>({});
  onMount(() => {
    initModules();
    getTranslationByLanguage().then((translations: Translations) => {
      setLanguages("en", translations);
    });
  });

  const value = createI18nContext(languages, "en");

  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <I18nContext.Provider value={value}>
          <App />
        </I18nContext.Provider>
      </Router>
    </rspc.Provider>
  );
}, document.getElementById("root") as HTMLElement);

render(() => {
  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <Modals />
      </Router>
    </rspc.Provider>
  );
}, document.getElementById("overlay") as HTMLElement);
