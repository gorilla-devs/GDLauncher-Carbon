/* @refresh reload */
import { onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import { client, queryClient, rspc } from "@/utils/rspcClient";
import { i18n, TransProvider, icu, loadLanguageFile } from "@gd/i18n";
import App from "@/app";
import Modals from "@/ModalsManager";
import initAnalytics from "@/utils/analytics";
import { initModules } from "@/modules";
import "virtual:uno.css";
import "virtual:unocss-devtools";
import "@gd/ui/style.css";

queueMicrotask(() => {
  initAnalytics();
});

const DEFAULT_LANG = "en";

const instance = i18n.createInstance({
  defaultNS: "common",
  fallbackLng: DEFAULT_LANG,
});
instance.use(icu);

loadLanguageFile(DEFAULT_LANG).then((langFile) => {
  instance.addResourceBundle(DEFAULT_LANG, "common", langFile);
});

render(() => {
  onMount(() => {
    initModules();
  });

  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <TransProvider instance={instance}>
          <App />
        </TransProvider>
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
