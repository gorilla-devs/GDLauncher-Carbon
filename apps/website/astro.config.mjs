/* eslint-disable import/no-default-export */
/* eslint-disable import/no-unresolved */
import solid from "@astrojs/solid-js";
import tailwindIntegration from "@astrojs/tailwind";
import { defineConfig } from "astro/config";
import Icons from "unplugin-icons/vite";

// https://astro.build/config
export default defineConfig({
  vite: {
    plugins: [
      Icons({
        compiler: "solid",
      }),
    ],
  },
  integrations: [solid(), tailwindIntegration()],
});
