import { spawn } from "child_process";
import { createServer, build } from "vite";
import electron from "@overwolf/ow-electron";

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
    VITE_DEV_MAIN_WINDOW_PORT: addressMainWindow.port
  });

  return build({
    configFile: "packages/main/vite.config.cjs",
    mode: "development",
    plugins: [
      {
        name: "electron-main-watcher",
        writeBundle() {
          electronProcess && electronProcess.kill();
          // Add "--inspect-brk=5858",  to debug main process
          electronProcess = spawn(electron, [".", "--test-ad"], {
            stdio: "inherit",
            env
          });
        }
      }
    ],
    build: {
      watch: true
    }
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
        }
      }
    ],
    build: {
      watch: true
    }
  });
}

const mainWindow = await createServer({
  configFile: "packages/mainWindow/vite.config.cjs"
});

await mainWindow.listen();
await watchPreload(mainWindow);
await watchMain(mainWindow);
