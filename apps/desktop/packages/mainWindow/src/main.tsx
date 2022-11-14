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
import * as Sentry from "@sentry/browser";
import { BrowserTracing } from "@sentry/tracing";

import { RewriteFrames as RewriteFramesIntegration } from "@sentry/integrations";

if (!import.meta.env.DEV) {
  const basePath = import.meta.url.split("app.asar")[0];
  Sentry.init({
    dsn: import.meta.env.VITE_SENTRY_DSN,
    integrations: [
      new BrowserTracing(),
      new RewriteFramesIntegration({
        iteratee: (frame) => {
          console.log(frame);
          if (frame.filename) {
            frame.filename = frame.filename.replace(basePath, "app:///");
          }
          return frame;
        },
      }),
    ],
    tracesSampleRate: 1.0,
    release: import.meta.env.VITE_PRECISE_VERSION,
  });
}

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
