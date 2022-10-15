/* @refresh reload */
import { createEffect, onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import App from "./app";
import Modals from "./Modals";
import "virtual:uno.css";
// import "virtual:unocss-devtools";
import "./utils/napi";
import initAnalytics from "./utils/analytics";
import { isModuleLoaded } from "./utils/napi";

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

console.log("ipcRenderer", window.ipcRenderer);

// Usage of ipcRenderer.on
window.ipcRenderer.on("main-process-message", (_event, ...args) => {
  console.log("[Receive Main-process message]:", ...args);
});
