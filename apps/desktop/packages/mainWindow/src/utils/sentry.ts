import * as Sentry from "@sentry/browser";
import { BrowserTracing } from "@sentry/tracing";
import { RewriteFrames as RewriteFramesIntegration } from "@sentry/integrations";

if (!import.meta.env.DEV) {
  try {
    const removeLastSection = (url: string) => {
      if (url.endsWith("/")) {
        url = url.slice(0, -1);
      }

      let sections = url.split("/");

      return sections.slice(0, sections.length - 1).join("/");
    };

    console.log("IMPORT META URL", import.meta.url);
    let basePath =
      "file://" +
      removeLastSection(import.meta.url.split("app.asar")[0]).replace(
        "file://",
        ""
      );
    console.log("BASE PATH", basePath);
    Sentry.init({
      dsn: import.meta.env.VITE_SENTRY_DSN,
      integrations: [
        new BrowserTracing(),
        new RewriteFramesIntegration({
          iteratee: (frame) => {
            if (frame.filename) {
              console.log("FRAME", frame.filename);
              frame.filename = frame.filename.replace(basePath, "app:/");
              console.log("FRAME AFTER", frame.filename);
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
