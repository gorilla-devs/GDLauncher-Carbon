import posthog from "posthog-js";
import { MetricEvent } from "./metricEvents";
import { FEOperatingSystem } from "@gd/core_module/bindings";

let isMetricsInitialized = false;

export async function initMetrics(os: FEOperatingSystem, randomUserId: string) {
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
    persistence: "memory",
    capture_heatmaps: false,
    autocapture: false,
    disable_session_recording: true,
    enable_heatmaps: false,
    capture_pageview: false,
    session_idle_timeout_seconds: 60 * 3,
    loaded: (posthog) => {
      if (process.env.NODE_ENV === "development") posthog.debug();
    },
    get_device_id: () => randomUserId
  });

  posthog.register_once({
    $initial_os: os.os,
    $initial_os_version: os.os_version,
    $initial_browser: "GDLauncher",
    $initial_browser_version: __APP_VERSION__
  });

  posthog.register({
    $os: os.os,
    $os_version: os.os_version,
    $browser: "GDLauncher",
    $browser_version: __APP_VERSION__
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
