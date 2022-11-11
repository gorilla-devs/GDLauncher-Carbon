import { resolve, join } from "path";
import { defineConfig, Plugin } from "vite";
import solidPlugin from "vite-plugin-solid";
import Unocss from "unocss/vite";
import { ViteMinifyPlugin } from "vite-plugin-minify";
import presetAttributify from "@unocss/preset-attributify";
import presetWind from "@unocss/preset-wind";
import pkg from "../../package.json";

/**
 * @see https://vitejs.dev/config/
 */
export default defineConfig({
  mode: process.env.NODE_ENV,
  root: __dirname,
  plugins: [
    solidPlugin(),
    Unocss({
      include: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
      presets: [
        presetAttributify({
          prefix: "uno:",
          prefixedOnly: true,
        }),
        presetWind(),
      ],
      rules: [
        [
          /^bg-image-(.*)$/,
          ([a, d]) => {
            let img = d.split("-")[0];
            let extension = a.split(".")[1];
            const isSvg = extension === "svg";
            return {
              background: `url('./${process.env.NODE_ENV === "development" ? "assets/" : ""
                }images/${isSvg ? img : `${img}.png`}')`,
              "background-size": "100% 100%",
              "background-repeat": "no-repeat",
              "box-sizing": "border-box",
            };
          },
        ],
      ],
      theme: {
        colors: {
          accent: {
            main: "#2B6CB0",
          },
        },
      },
    }),
    ViteMinifyPlugin({}),
  ],
  envDir: resolve(__dirname, "../../../../"),
  base: "./",
  build: {
    target: "esnext",
    emptyOutDir: true,
    outDir: "../../dist/mainWindow",
    sourcemap: true,
  },
  resolve: {
    alias: {
      "@": join(__dirname, "src"),
    },
  },
  server: {
    port: pkg.env.PORT,
  },
});
