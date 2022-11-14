import * as Sentry from "@sentry/browser";
import { BrowserTracing } from "@sentry/tracing";
import { RewriteFrames as RewriteFramesIntegration } from "@sentry/integrations";

if (!import.meta.env.DEV) {
  try {
    let basePath =
      "file://" +
      window.nodePath.resolve(
        import.meta.url.split("app.asar")[0].replace("file://", ""),
        "../"
      );
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
  } catch (e) {
    console.error(e);
  }
}
