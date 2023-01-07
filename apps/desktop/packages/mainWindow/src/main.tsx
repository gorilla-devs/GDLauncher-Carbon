/* @refresh reload */
import { createSignal, onMount } from "solid-js";
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
import { LanguagesProvider } from "./languagesProvider";

queueMicrotask(() => {
  initAnalytics();
});

render(() => {
  onMount(() => {
    initModules();
  });

  // const dict = {
  //   in: {
  //     hello: "bonjour {{ name }}, comment vas-tu ?",
  //   },
  //   en: {
  //     hello: "hello {{ name }}, how are you?",
  //   },
  // };

  // const value = createI18nContext(dict, "fr");

  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <LanguagesProvider>
          <App />
        </LanguagesProvider>
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
