import posthog from "posthog-js";

let init = false;
function initAnalytics(metricsLevel: Number) {
  if (init) {
    return;
  }

  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    posthog.init(import.meta.env.VITE_POSTHOG_KEY, {
      api_host: import.meta.env.VITE_METRICS_URL,
      disable_session_recording: metricsLevel === 1,
      loaded: async () => {
        let os = await window.getCurrentOS();

        posthog.register({
          $set: {
            app_version: __APP_VERSION__,
            os: os.platform,
            arch: os.arch,
          },
        });
      },
    });
  }

  init = true;
}

export function trackEvent(event: string, properties?: Record<string, any>) {
  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    posthog.capture(event, {
      ...(properties || {}),
    });
  }
}

export default initAnalytics;
