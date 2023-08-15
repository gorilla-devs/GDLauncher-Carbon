import { build } from "vite";

await build({
  configFile: "packages/main/vite.config.cjs",
  mode: "production",
});
await build({
  configFile: "packages/preload/vite.config.js",
  mode: "production",
});
await build({
  configFile: "packages/mainWindow/vite.config.cjs",
  mode: "production",
  optimizeDeps: true,
});
