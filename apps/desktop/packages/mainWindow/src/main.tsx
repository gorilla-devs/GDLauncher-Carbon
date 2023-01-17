/* @refresh reload */
import { onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import { client, queryClient, rspc } from "@/utils/rspcClient";
import App from "@/app";
import "@/utils/theme";
import Modals from "@/ModalsManager";
import initAnalytics from "@/utils/analytics";
import "virtual:uno.css";
import "@gd/ui/style.css";
import { NotificationsProvider } from "@gd/ui";

queueMicrotask(() => {
  initAnalytics();
});

render(() => {
  onMount(() => {
    window.clearLoading();
  });

  return (
    <rspc.Provider client={client as any} queryClient={queryClient}>
      <Router source={hashIntegration()}>
        <NotificationsProvider>
          <App />
        </NotificationsProvider>
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
