/* eslint-disable import/no-default-export */

/* eslint-disable import/no-unresolved */
import solid from "@astrojs/solid-js";
import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";
import mdx from "@astrojs/mdx";

// https://astro.build/config
import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  vite: {
    // envDir: resolve(__dirname, "../../"),
    ssr: {
      // noExternal: "style.css", // from @gd/ui
    },
  },
  site: "https://gdlauncher.com",
  integrations: [
    solid(),
    tailwind({
      config: {
        applyBaseStyles: false,
      },
    }),
    mdx(),
    sitemap({
      filter: (page) => page !== "https://gdlauncher.com/newsletter/confirm",
      i18n: {
        defaultLocale: "en",
        locales: {
          en: "en-US",
        },
      },
    }),
  ],
});
