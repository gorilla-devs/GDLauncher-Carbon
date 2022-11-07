/* @refresh reload */
import { createEffect, onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import App from "./app";
import Modals from "./Modals";
import "virtual:uno.css";
import "virtual:unocss-devtools";
import "./utils/napi";
import initAnalytics from "./utils/analytics";
import { isModuleLoaded } from "./utils/napi";
import loadHandlers from "./utils/loadHandlers";

queueMicrotask(() => {
  initAnalytics();
});

render(() => {
  createEffect(() => {
    console.log("isModuleLoaded", isModuleLoaded());
    if (isModuleLoaded() === true) {
      window.clearState();
    } else if (isModuleLoaded() instanceof Error) {
      window.fatalError(isModuleLoaded() as Error);
    }
  });

  onMount(() => {
    loadHandlers();
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
