import { build } from "vite";

await build({ configFile: "packages/main/vite.config.ts", mode: "production" });
await build({
  configFile: "packages/preload/vite.config.ts",
  mode: "production",
});
await build({
  configFile: "packages/mainWindow/vite.config.ts",
  mode: "production",
  optimizeDeps: true,
});