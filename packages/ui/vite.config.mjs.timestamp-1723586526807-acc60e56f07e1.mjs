// vite.config.mjs
import path, { resolve } from "path";
import { defineConfig } from "file:///Users/davideceschia/Documents/gitRepos/GDLauncher-Carbon/node_modules/.pnpm/vite@4.5.2_@types+node@20.11.6_less@4.2.0/node_modules/vite/dist/node/index.js";
import solidPlugin from "file:///Users/davideceschia/Documents/gitRepos/GDLauncher-Carbon/node_modules/.pnpm/vite-plugin-solid@2.9.1_solid-js@1.8.15_vite@4.5.2/node_modules/vite-plugin-solid/dist/esm/index.mjs";
import dts from "file:///Users/davideceschia/Documents/gitRepos/GDLauncher-Carbon/node_modules/.pnpm/vite-plugin-dts@2.3.0_vite@4.5.2/node_modules/vite-plugin-dts/dist/index.mjs";
import Unocss from "file:///Users/davideceschia/Documents/gitRepos/GDLauncher-Carbon/node_modules/.pnpm/unocss@0.60.4_postcss@8.4.35_vite@4.5.2/node_modules/unocss/dist/vite.mjs";
import { unocssConfig } from "file:///Users/davideceschia/Documents/gitRepos/GDLauncher-Carbon/packages/config/dist/index.js";
import { dirname } from "path";
import { fileURLToPath } from "url";
var __vite_injected_original_import_meta_url = "file:///Users/davideceschia/Documents/gitRepos/GDLauncher-Carbon/packages/ui/vite.config.mjs";
var __dirname = dirname(fileURLToPath(__vite_injected_original_import_meta_url));
var vite_config_default = defineConfig({
  plugins: [
    dts({
      insertTypesEntry: false,
      tsConfigFilePath: resolve(__dirname, "tsconfig.json")
    }),
    solidPlugin(),
    process.env.STORYBOOK && Unocss(unocssConfig)
  ],
  test: {
    globals: true,
    environment: "jsdom",
    transformMode: {
      web: [/\.[t|s]sx?$/]
    },
    setupFiles: "./setupVitest.js",
    // solid needs to be inline to work around
    // a resolution issue in vitest
    // And solid-testing-library needs to be here so that the 'hydrate'
    // method will be provided
    deps: {
      inline: [/solid-js/]
    }
  },
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      name: "@gd/ui",
      formats: ["es"],
      fileName: (format) => `ui.${format}.js`
    },
    rollupOptions: {
      external: ["solid-js"],
      output: {
        globals: {
          "solid-js": "SolidJS"
        }
      }
    },
    target: "esnext",
    emptyOutDir: false
  },
  resolve: {
    alias: {
      "@": path.join(__dirname, "src")
    }
  }
});
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcubWpzIl0sCiAgInNvdXJjZXNDb250ZW50IjogWyJjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZGlybmFtZSA9IFwiL1VzZXJzL2RhdmlkZWNlc2NoaWEvRG9jdW1lbnRzL2dpdFJlcG9zL0dETGF1bmNoZXItQ2FyYm9uL3BhY2thZ2VzL3VpXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ZpbGVuYW1lID0gXCIvVXNlcnMvZGF2aWRlY2VzY2hpYS9Eb2N1bWVudHMvZ2l0UmVwb3MvR0RMYXVuY2hlci1DYXJib24vcGFja2FnZXMvdWkvdml0ZS5jb25maWcubWpzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy9Vc2Vycy9kYXZpZGVjZXNjaGlhL0RvY3VtZW50cy9naXRSZXBvcy9HRExhdW5jaGVyLUNhcmJvbi9wYWNrYWdlcy91aS92aXRlLmNvbmZpZy5tanNcIjtpbXBvcnQgcGF0aCwgeyByZXNvbHZlIH0gZnJvbSBcInBhdGhcIjtcbmltcG9ydCB7IGRlZmluZUNvbmZpZyB9IGZyb20gXCJ2aXRlXCI7XG5pbXBvcnQgc29saWRQbHVnaW4gZnJvbSBcInZpdGUtcGx1Z2luLXNvbGlkXCI7XG5pbXBvcnQgZHRzIGZyb20gXCJ2aXRlLXBsdWdpbi1kdHNcIjtcbmltcG9ydCBVbm9jc3MgZnJvbSBcInVub2Nzcy92aXRlXCI7XG5pbXBvcnQgeyB1bm9jc3NDb25maWcgfSBmcm9tIFwiQGdkL2NvbmZpZ1wiO1xuaW1wb3J0IHsgZGlybmFtZSB9IGZyb20gXCJwYXRoXCI7XG5pbXBvcnQgeyBmaWxlVVJMVG9QYXRoIH0gZnJvbSBcInVybFwiO1xuXG5jb25zdCBfX2Rpcm5hbWUgPSBkaXJuYW1lKGZpbGVVUkxUb1BhdGgoaW1wb3J0Lm1ldGEudXJsKSk7XG5cbmV4cG9ydCBkZWZhdWx0IGRlZmluZUNvbmZpZyh7XG4gIHBsdWdpbnM6IFtcbiAgICBkdHMoe1xuICAgICAgaW5zZXJ0VHlwZXNFbnRyeTogZmFsc2UsXG4gICAgICB0c0NvbmZpZ0ZpbGVQYXRoOiByZXNvbHZlKF9fZGlybmFtZSwgXCJ0c2NvbmZpZy5qc29uXCIpLFxuICAgIH0pLFxuICAgIHNvbGlkUGx1Z2luKCksXG4gICAgcHJvY2Vzcy5lbnYuU1RPUllCT09LICYmIFVub2Nzcyh1bm9jc3NDb25maWcpLFxuICBdLFxuICB0ZXN0OiB7XG4gICAgZ2xvYmFsczogdHJ1ZSxcbiAgICBlbnZpcm9ubWVudDogXCJqc2RvbVwiLFxuICAgIHRyYW5zZm9ybU1vZGU6IHtcbiAgICAgIHdlYjogWy9cXC5bdHxzXXN4PyQvXSxcbiAgICB9LFxuICAgIHNldHVwRmlsZXM6IFwiLi9zZXR1cFZpdGVzdC5qc1wiLFxuICAgIC8vIHNvbGlkIG5lZWRzIHRvIGJlIGlubGluZSB0byB3b3JrIGFyb3VuZFxuICAgIC8vIGEgcmVzb2x1dGlvbiBpc3N1ZSBpbiB2aXRlc3RcbiAgICAvLyBBbmQgc29saWQtdGVzdGluZy1saWJyYXJ5IG5lZWRzIHRvIGJlIGhlcmUgc28gdGhhdCB0aGUgJ2h5ZHJhdGUnXG4gICAgLy8gbWV0aG9kIHdpbGwgYmUgcHJvdmlkZWRcbiAgICBkZXBzOiB7XG4gICAgICBpbmxpbmU6IFsvc29saWQtanMvXSxcbiAgICB9LFxuICB9LFxuICBidWlsZDoge1xuICAgIGxpYjoge1xuICAgICAgZW50cnk6IHBhdGgucmVzb2x2ZShfX2Rpcm5hbWUsIFwic3JjL2luZGV4LnRzXCIpLFxuICAgICAgbmFtZTogXCJAZ2QvdWlcIixcbiAgICAgIGZvcm1hdHM6IFtcImVzXCJdLFxuICAgICAgZmlsZU5hbWU6IChmb3JtYXQpID0+IGB1aS4ke2Zvcm1hdH0uanNgLFxuICAgIH0sXG4gICAgcm9sbHVwT3B0aW9uczoge1xuICAgICAgZXh0ZXJuYWw6IFtcInNvbGlkLWpzXCJdLFxuICAgICAgb3V0cHV0OiB7XG4gICAgICAgIGdsb2JhbHM6IHtcbiAgICAgICAgICBcInNvbGlkLWpzXCI6IFwiU29saWRKU1wiLFxuICAgICAgICB9LFxuICAgICAgfSxcbiAgICB9LFxuICAgIHRhcmdldDogXCJlc25leHRcIixcbiAgICBlbXB0eU91dERpcjogZmFsc2UsXG4gIH0sXG4gIHJlc29sdmU6IHtcbiAgICBhbGlhczoge1xuICAgICAgXCJAXCI6IHBhdGguam9pbihfX2Rpcm5hbWUsIFwic3JjXCIpLFxuICAgIH0sXG4gIH0sXG59KTtcbiJdLAogICJtYXBwaW5ncyI6ICI7QUFBbVksT0FBTyxRQUFRLGVBQWU7QUFDamEsU0FBUyxvQkFBb0I7QUFDN0IsT0FBTyxpQkFBaUI7QUFDeEIsT0FBTyxTQUFTO0FBQ2hCLE9BQU8sWUFBWTtBQUNuQixTQUFTLG9CQUFvQjtBQUM3QixTQUFTLGVBQWU7QUFDeEIsU0FBUyxxQkFBcUI7QUFQcU4sSUFBTSwyQ0FBMkM7QUFTcFMsSUFBTSxZQUFZLFFBQVEsY0FBYyx3Q0FBZSxDQUFDO0FBRXhELElBQU8sc0JBQVEsYUFBYTtBQUFBLEVBQzFCLFNBQVM7QUFBQSxJQUNQLElBQUk7QUFBQSxNQUNGLGtCQUFrQjtBQUFBLE1BQ2xCLGtCQUFrQixRQUFRLFdBQVcsZUFBZTtBQUFBLElBQ3RELENBQUM7QUFBQSxJQUNELFlBQVk7QUFBQSxJQUNaLFFBQVEsSUFBSSxhQUFhLE9BQU8sWUFBWTtBQUFBLEVBQzlDO0FBQUEsRUFDQSxNQUFNO0FBQUEsSUFDSixTQUFTO0FBQUEsSUFDVCxhQUFhO0FBQUEsSUFDYixlQUFlO0FBQUEsTUFDYixLQUFLLENBQUMsYUFBYTtBQUFBLElBQ3JCO0FBQUEsSUFDQSxZQUFZO0FBQUE7QUFBQTtBQUFBO0FBQUE7QUFBQSxJQUtaLE1BQU07QUFBQSxNQUNKLFFBQVEsQ0FBQyxVQUFVO0FBQUEsSUFDckI7QUFBQSxFQUNGO0FBQUEsRUFDQSxPQUFPO0FBQUEsSUFDTCxLQUFLO0FBQUEsTUFDSCxPQUFPLEtBQUssUUFBUSxXQUFXLGNBQWM7QUFBQSxNQUM3QyxNQUFNO0FBQUEsTUFDTixTQUFTLENBQUMsSUFBSTtBQUFBLE1BQ2QsVUFBVSxDQUFDLFdBQVcsTUFBTSxNQUFNO0FBQUEsSUFDcEM7QUFBQSxJQUNBLGVBQWU7QUFBQSxNQUNiLFVBQVUsQ0FBQyxVQUFVO0FBQUEsTUFDckIsUUFBUTtBQUFBLFFBQ04sU0FBUztBQUFBLFVBQ1AsWUFBWTtBQUFBLFFBQ2Q7QUFBQSxNQUNGO0FBQUEsSUFDRjtBQUFBLElBQ0EsUUFBUTtBQUFBLElBQ1IsYUFBYTtBQUFBLEVBQ2Y7QUFBQSxFQUNBLFNBQVM7QUFBQSxJQUNQLE9BQU87QUFBQSxNQUNMLEtBQUssS0FBSyxLQUFLLFdBQVcsS0FBSztBQUFBLElBQ2pDO0FBQUEsRUFDRjtBQUNGLENBQUM7IiwKICAibmFtZXMiOiBbXQp9Cg==
