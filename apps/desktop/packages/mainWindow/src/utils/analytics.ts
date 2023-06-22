import posthog from "posthog-js";

function initAnalytics() {
  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    posthog.init(import.meta.env.VITE_POSTHOG_KEY, {
      api_host: import.meta.env.VITE_METRICS_URL,
      disable_session_recording: false,
      debug: true,
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
}

export function trackEvent(event: string, properties?: Record<string, any>) {
  if (import.meta.env.VITE_POSTHOG_KEY && import.meta.env.VITE_METRICS_URL) {
    posthog.capture(event, {
      ...(properties || {}),
    });
  }
}

export default initAnalytics;
