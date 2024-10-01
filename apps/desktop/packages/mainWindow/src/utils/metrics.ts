import posthog from "posthog-js";
import { MetricEvent } from "./metricEvents";

let isMetricsInitialized = false;

export function initMetrics() {
  if (isMetricsInitialized) {
    return;
  }

  if (!import.meta.env.VITE_POSTHOG_KEY) {
    console.warn("Posthog key not found");
    return;
  }

  isMetricsInitialized = true;

  posthog.init(import.meta.env.VITE_POSTHOG_KEY, {
    api_host: "https://eu.i.posthog.com",
    person_profiles: "identified_only",
    capture_heatmaps: true,
    autocapture: false,
    disable_session_recording: true,
    enable_heatmaps: true,
    capture_pageview: false,
    loaded: (posthog) => {
      if (process.env.NODE_ENV === "development") posthog.debug();
    }
  });

  function getPatchedUrl() {
    let patchedUrl = window.location.hash;
    if (patchedUrl.startsWith("#/")) {
      patchedUrl = patchedUrl.replace("#/", "/");
    }
    return patchedUrl;
  }

  const patchedUrl = getPatchedUrl();
  posthog.capture("$pageview", {
    $current_url: patchedUrl
  });

  window.addEventListener("hashchange", () => {
    const patchedUrl = getPatchedUrl();
    posthog.capture("$pageview", {
      $current_url: patchedUrl
    });
  });

  posthog.capture(MetricEvent.LauncherStarted);
}
