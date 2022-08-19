import { spawn } from "child_process";
import { createServer, build } from "vite";
import electron from "electron";

/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchMain(mainWindow, mcLogWindow) {
  /**
   * @type {import('child_process').ChildProcessWithoutNullStreams | null}
   */
  let electronProcess = null;
  const addressMainWindow = mainWindow.httpServer.address();
  const addressMcLogWindow = mcLogWindow.httpServer.address();
  const env = Object.assign(process.env, {
    VITE_DEV_SERVER_HOST: "localhost",
    VITE_DEV_MAIN_WINDOW_PORT: addressMainWindow.port,
    VITE_DEV_MC_LOG_WINDOW_PORT: addressMcLogWindow.port,
  });

  return build({
    configFile: "packages/main/vite.config.ts",
    mode: "development",
    plugins: [
      {
        name: "electron-main-watcher",
        writeBundle() {
          electronProcess && electronProcess.kill();
          electronProcess = spawn(electron, ["."], { stdio: "inherit", env });
        },
      },
    ],
    build: {
      watch: true,
    },
  });
}

/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchPreload(mainWindow, mcLogWindow) {
  return build({
    configFile: "packages/preload/vite.config.ts",
    mode: "development",
    plugins: [
      {
        name: "electron-preload-watcher",
        writeBundle() {
          mainWindow.ws.send({ type: "full-reload" });
          mcLogWindow.ws.send({ type: "full-reload" });
        },
      },
    ],
    build: {
      watch: true,
    },
  });
}

// bootstrap
const mainWindow = await createServer({
  configFile: "packages/mainWindow/vite.config.ts",
});
const mcLogWindow = await createServer({
  configFile: "packages/mcLogWindow/vite.config.ts",
});

await mainWindow.listen();
await mcLogWindow.listen();
await watchPreload(mainWindow, mcLogWindow);
await watchMain(mainWindow, mcLogWindow);
