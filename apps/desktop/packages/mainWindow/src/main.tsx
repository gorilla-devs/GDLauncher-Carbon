/* @refresh reload */
import { createEffect, onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import App from "./app";
import Modals from "./Modals";
import "virtual:uno.css";
import "virtual:unocss-devtools";
import initAnalytics from "./utils/analytics";
import { initModules } from "./modules";
import "./utils/sentry";

queueMicrotask(() => {
  initAnalytics();
});

render(() => {
  onMount(() => {
    initModules();
  });

  return (
    <Router source={hashIntegration()}>
      <App />
    </Router>
  );
}, document.getElementById("root") as HTMLElement);

render(() => {
  return (
    <Router source={hashIntegration()}>
      <Modals />
    </Router>
  );
}, document.getElementById("overlay") as HTMLElement);
