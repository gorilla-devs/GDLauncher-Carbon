import { defineConfig } from "vite"
import solid from "vite-plugin-solid"
import { resolve } from "path"
import { TanStackRouterVite } from "@tanstack/router-vite-plugin"
import Unocss from "unocss/vite"
import { unocssConfig } from "../../config/src/unocss.config.ts"
import { dirname } from "path"
import { fileURLToPath } from "url"

const __dirname = dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [
    TanStackRouterVite({
      target: "solid",
      routesDirectory: resolve(__dirname, "src/routes"),
      generatedRouteTree: resolve(__dirname, "src/routeTree.gen.ts"),
      quoteStyle: "single",
      semicolons: false
    }),
    solid(),
    Unocss(unocssConfig)
  ],
  server: {
    port: 3001
  },
  root: resolve(__dirname, "."),
  publicDir: resolve(__dirname, "public"),
  build: {
    outDir: resolve(__dirname, "dist")
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "src")
    }
  }
})
