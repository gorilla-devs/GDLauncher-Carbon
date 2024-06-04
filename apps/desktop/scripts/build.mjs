import { build } from "vite";

await build({
  configFile: "packages/main/vite.config.mjs",
  mode: "production"
});
await build({
  configFile: "packages/preload/vite.config.mjs",
  mode: "production"
});
await build({
  configFile: "packages/mainWindow/vite.config.mjs",
  mode: "production",
  optimizeDeps: true
});
