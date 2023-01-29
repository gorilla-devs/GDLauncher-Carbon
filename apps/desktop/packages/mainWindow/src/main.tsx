/* @refresh reload */
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import { client, queryClient, rspc } from "@/utils/rspcClient";
import { i18n, TransProvider, icu, loadLanguageFile } from "@gd/i18n";
import App from "@/app";
import Modals from "@/ModalsManager";
import initAnalytics from "@/utils/analytics";
import "virtual:uno.css";
import "@gd/ui/style.css";
import { NotificationsProvider } from "@gd/ui";

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
  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <TransProvider instance={instance}>
          <NotificationsProvider>
            <App />
          </NotificationsProvider>
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
