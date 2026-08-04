import { spawn } from "child_process"
import { createServer, build } from "vite"
import electron from "@overwolf/ow-electron"
import dotenv from "dotenv"

// The spawned Electron process inherits process.env, so the root .env has to be
// loaded before it is read below. ow-electron reads OW_CLI_EMAIL/OW_CLI_API_KEY
// from the environment to authenticate dev mode, without which the gaming
// packages (GEP, Overlay, Recorder) do not load.
dotenv.config({
  path: "../../.env",
  quiet: true
})

let electronProcess = null
/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchMain(mainWindow) {
  /**
   * @type {import('child_process').ChildProcessWithoutNullStreams | null}
   */
  const addressMainWindow = mainWindow.httpServer.address()
  const env = Object.assign(process.env, {
    VITE_DEV_SERVER_HOST: "localhost",
    VITE_DEV_MAIN_WINDOW_PORT: addressMainWindow.port
  })

  return build({
    configFile: "packages/main/vite.config.mjs",
    mode: "development",
    plugins: [
      {
        name: "electron-main-watcher",
        writeBundle() {
          // eslint-disable-next-line @typescript-eslint/no-unused-expressions
          electronProcess && electronProcess.kill()
          // Add "--inspect-brk=5858",  to debug main process
          electronProcess = spawn(electron, [".", "--test-ad"], {
            stdio: "inherit",
            env
          })
        }
      }
    ],
    build: {
      watch: true
    }
  })
}

/**
 * @type {(server: import('vite').ViteDevServer) => Promise<import('rollup').RollupWatcher>}
 */
function watchPreload(mainWindow) {
  return build({
    configFile: "packages/preload/vite.config.mjs",
    mode: "development",
    plugins: [
      {
        name: "electron-preload-watcher",
        writeBundle() {
          mainWindow.ws.send({ type: "full-reload" })
        }
      }
    ],
    build: {
      watch: true
    }
  })
}

const mainWindow = await createServer({
  configFile: "packages/mainWindow/vite.config.mjs"
})

await mainWindow.listen()
await watchPreload(mainWindow)
await watchMain(mainWindow)
