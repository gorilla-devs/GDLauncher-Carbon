import posthog from "posthog-js";
import { rspc } from "./rspcClient";
import { createEffect } from "solid-js";

export let init = false;

let startupEventSent = false;

let backlog: Array<[string, any]> = [];

function clearBacklog() {
  console.log("Clearing metrics backlog");
  backlog.forEach(([eventName, properties]) => {
    posthog.capture(eventName, properties);
  });
  console.log("Metrics backlog cleared");

  backlog = [];
}

function initAnalytics() {
  if (
    import.meta.env.VITE_POSTHOG_KEY &&
    import.meta.env.VITE_METRICS_URL &&
    !init
  ) {
    let settings = rspc.createQuery(() => ["settings.getSettings"]);

    createEffect(() => {
      if (!settings.data) return;

      init = true;
      posthog.init(import.meta.env.VITE_POSTHOG_KEY, {
        api_host: import.meta.env.VITE_METRICS_URL,
        disable_session_recording: true,
        autocapture: false,
        persistence: "memory",
        debug: import.meta.env.DEV,
        opt_out_capturing_by_default: !settings.data.metricsEnabled,
        bootstrap: {
          distinctID: settings.data.randomUserUuid,
        },
        loaded: async () => {
          const os = await window.getCurrentOS();

          posthog.register({
            $set: {
              app_version: __APP_VERSION__,
              os: os.platform,
              arch: os.arch,
            },
            $pathname: window.location.hash,
            $current_url: window.location.hash,
          });

          if (!startupEventSent && settings.data?.metricsEnabled) {
            clearBacklog();
            startupEventSent = true;
            trackEvent("app_started");
          }
        },
      });
    });

    createEffect(() => {
      if (!settings.data) return;

      if (settings.data?.metricsEnabled && !posthog.has_opted_in_capturing()) {
        posthog.opt_in_capturing({
          capture_properties: {
            $current_url: window.location.hash,
            $pathname: window.location.hash,
          },
        });

        if (!startupEventSent) {
          clearBacklog();
          startupEventSent = true;
          trackEvent("app_started");
        }
      } else if (
        !settings.data?.metricsEnabled &&
        posthog.has_opted_in_capturing()
      ) {
        posthog.opt_out_capturing();
      }
    });
  }
}

window.addEventListener("hashchange", trackPageView);

export function trackEvent(event: string, properties?: Record<string, any>) {
  if (
    import.meta.env.VITE_POSTHOG_KEY &&
    import.meta.env.VITE_METRICS_URL &&
    posthog.has_opted_in_capturing()
  ) {
    if (!startupEventSent) {
      backlog.push([
        event,
        {
          ...(properties || {}),
          $pathname: window.location.hash,
          $current_url: window.location.hash,
        },
      ]);
    } else {
      posthog.capture(event, {
        ...(properties || {}),
      });
    }
  }
}

export function trackPageView() {
  if (
    import.meta.env.VITE_POSTHOG_KEY &&
    import.meta.env.VITE_METRICS_URL &&
    posthog.has_opted_in_capturing()
  ) {
    if (!startupEventSent) {
      backlog.push([
        "$pageview",
        {
          $pathname: window.location.hash,
          $current_url: window.location.hash,
        },
      ]);
    } else {
      posthog.capture("$pageview", {
        $pathname: window.location.hash,
        $current_url: window.location.hash,
      });
    }
  }
}

export default initAnalytics;
