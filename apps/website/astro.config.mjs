/* eslint-disable import/no-default-export */

/* eslint-disable import/no-unresolved */
import solid from "@astrojs/solid-js";
import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";

import mdx from "@astrojs/mdx";

// https://astro.build/config
export default defineConfig({
  vite: {
    // envDir: resolve(__dirname, "../../"),
    ssr: {
      // noExternal: "style.css", // from @gd/ui
    },
  },
  integrations: [
    solid(),
    tailwind({ config: { applyBaseStyles: false } }),
    mdx(),
  ],
});
