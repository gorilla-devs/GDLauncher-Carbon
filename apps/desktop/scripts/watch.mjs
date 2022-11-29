import { spawn } from "child_process";
import { createServer, build } from "vite";
import electron from "electron";
import chokidar from "chokidar";

let electronProcess = null;
/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchMain(mainWindow) {
  /**
   * @type {import('child_process').ChildProcessWithoutNullStreams | null}
   */
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

  const addressMainWindow = mainWindow.httpServer.address();
  const env = Object.assign(process.env, {
    VITE_DEV_SERVER_HOST: "localhost",
    VITE_DEV_MAIN_WINDOW_PORT: addressMainWindow.port,
  });

  chokidar
    .watch("../../packages/core/core.node", { ignoreInitial: true })
    .on("all", (event, path) => {
      console.log("Reloading app due to native core rebuild", event);
      electronProcess?.kill();
      electronProcess = spawn(electron, ["."], { stdio: "inherit", env });
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

//! IMPORTANT !//
// This only works on Unix systems, as in windows you cannot delete a file while it's being used (core.node).
// The workaround for windows would be to build the native core to a different file, and then reload the app
// targetting the new file.
if (process.platform !== "win32") {
  await watchNativeCore(mainWindow);
}
