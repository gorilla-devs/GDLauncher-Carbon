import * as Sentry from "@sentry/browser";
import { BrowserTracing } from "@sentry/tracing";
import { RewriteFrames as RewriteFramesIntegration } from "@sentry/integrations";
import { getBasePathUrl } from ".";

if (!import.meta.env.DEV) {
  try {
    let basePath = getBasePathUrl(import.meta.url);
    Sentry.init({
      dsn: import.meta.env.VITE_SENTRY_DSN,
      initialScope: {
        tags: {
          baseUrl: import.meta.url,
        },
      },
      integrations: [
        new BrowserTracing(),
        new RewriteFramesIntegration({
          iteratee: (frame) => {
            if (frame.filename) {
              frame.filename = frame.filename
                .replace(basePath, "app:/")
                .toLowerCase();
            }
            return frame;
          },
        }),
      ],
      tracesSampleRate: 1.0,
      release: import.meta.env.VITE_PRECISE_VERSION,
    });
  } catch (e) {
    console.error(e);
  }
}
