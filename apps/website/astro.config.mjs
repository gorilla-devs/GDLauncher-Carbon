/* eslint-disable import/no-default-export */

/* eslint-disable import/no-unresolved */
import solid from "@astrojs/solid-js";
import Unocss from "unocss/astro";
import presetWind from "@unocss/preset-wind";
import presetAttributify from "@unocss/preset-attributify";
import { defineConfig } from "astro/config";
import uno from "astro-uno";
import { presetUno } from "unocss";

import mdx from "@astrojs/mdx";

// https://astro.build/config
export default defineConfig({
  vite: {
    // envDir: resolve(__dirname, "../../"),
    ssr: {// noExternal: "style.css", // from @gd/ui
    }
  },
  integrations: [solid(), Unocss({
    presets: [presetWind(), presetAttributify()]
  }), uno({
    presets: [presetUno()]
  }), mdx()]
});