/* @refresh reload */
import { onMount } from "solid-js";
import { render } from "solid-js/web";
import { Router, hashIntegration } from "@solidjs/router";
import { ThemeProvider } from "solid-styled-components";
import App from "./app";
import Modals from "./Modals";
import initAnalytics from "./utils/analytics";
import { initModules } from "./modules";
import { theme } from "@gd/ui";

queueMicrotask(() => {
        initAnalytics();
});

render(() => {
  onMount(() => {
    initModules();
  });

  return (
    <ThemeProvider theme={theme}>
      <Router source={hashIntegration()}>
        <App />
      </Router>
    </ThemeProvider>
  );
}, document.getElementById("root") as HTMLElement);

render(() => {
  return (
    <ThemeProvider theme={theme}>
      <Router source={hashIntegration()}>
        <Modals />
      </Router>
    </ThemeProvider>
  );
}, document.getElementById("overlay") as HTMLElement);
