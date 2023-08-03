import posthog from "posthog-js";
import { rspc } from "./rspcClient";
import { createEffect } from "solid-js";

export let init = false;

let startupEventSent = false;

function initAnalytics() {
  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    let settings = rspc.createQuery(() => ["settings.getSettings"]);

    createEffect(() => {
      if (!settings.data) return;

      init = true;
      posthog.init(import.meta.env.VITE_POSTHOG_KEY, {
        api_host: import.meta.env.VITE_METRICS_URL,
        disable_session_recording: true,
        autocapture: false,
        persistence: "memory",
        // debug: import.meta.env.DEV,
        debug: true,
        opt_out_capturing_by_default: !settings.data.metricsEnabled,
        bootstrap: {
          distinctID: settings.data.randomUserUuid,
        },
        loaded: async () => {
          let os = await window.getCurrentOS();

          posthog.register({
            $set: {
              app_version: __APP_VERSION__,
              os: os.platform,
              arch: os.arch,
            },
          });

          if (!startupEventSent && settings.data?.metricsEnabled) {
            startupEventSent = true;
            trackEvent("app_started");
          }
        },
      });
    });

    createEffect(() => {
      if (!settings.data) return;

      if (settings.data?.metricsEnabled) {
        posthog.opt_in_capturing();

        if (!startupEventSent) {
          startupEventSent = true;
          trackEvent("app_started");
        }
      } else {
        posthog.opt_out_capturing();
      }
    });
  }
}

export function trackEvent(event: string, properties?: Record<string, any>) {
  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    posthog.capture(event, {
      ...(properties || {}),
    });
  }
}

export function trackPageView() {
  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    posthog.capture("$pageview");
  }
}

window.addEventListener("hashchange", trackPageView);

export default initAnalytics;
