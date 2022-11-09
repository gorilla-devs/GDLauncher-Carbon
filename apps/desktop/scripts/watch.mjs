import { spawn } from "child_process";
import { createServer, build } from "vite";
import electron from "electron";
import chokidar from "chokidar";

/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchMain(mainWindow) {
  /**
   * @type {import('child_process').ChildProcessWithoutNullStreams | null}
   */
  let electronProcess = null;
  const addressMainWindow = mainWindow.httpServer.address();
  const env = Object.assign(process.env, {
    VITE_DEV_SERVER_HOST: "localhost",
    VITE_DEV_MAIN_WINDOW_PORT: addressMainWindow.port,
  });

  return build({
    configFile: "packages/main/vite.config.js",
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

function watchNativeCore(mainWindow) {
  spawn("pnpm", ["watch"], {
    cwd: "../../packages/core",
    shell: true,
    stdio: "inherit",
  });

  chokidar
    .watch("../../packages/core/core.node", { ignoreInitial: true })
    .on("all", (event, path) => {
      console.log("Reloading app due to native core rebuild", event);
      mainWindow.ws.send({ type: "full-reload" });
    });
}
/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchPreload(mainWindow) {
  return build({
    configFile: "packages/preload/vite.config.js",
    mode: "development",
    plugins: [
      {
        name: "electron-preload-watcher",
        writeBundle() {
          mainWindow.ws.send({ type: "full-reload" });
        },
      },
    ],
    build: {
      watch: true,
    },
  });
}

const mainWindow = await createServer({
  configFile: "packages/mainWindow/vite.config.js",
});

await mainWindow.listen();
await watchPreload(mainWindow);
await watchMain(mainWindow);
await watchNativeCore(mainWindow);
