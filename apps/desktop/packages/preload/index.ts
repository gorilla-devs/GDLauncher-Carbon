import { contextBridge, ipcRenderer } from "electron"
import { subscribeProtocol } from "./protocolBridge"
import {
  subscribeCloseWarning,
  type CloseWarningReasons
} from "./closeWarningBridge"
import { subscribeShutdownPrompt } from "./shutdownPromptBridge"
import { coreExit, subscribeCoreExit, type CoreExit } from "./coreExitBridge"

import "./loading"
import "./core_module_loader"
import "./minimumBounds"
import "./autoupdate"
import "./openExternal"
import "./os"
import "./overwolf"
import "./runtimePath"
import "./relaunch"
import "./adViewHost"
import "./themeHost"

const skipIntroAnimation =
  process.argv
    .find((arg) => arg.startsWith("--skip-intro-animation="))
    ?.split("=")[1] === "true"

const isDev = import.meta.env.DEV

if (isDev || skipIntroAnimation) {
  contextBridge.exposeInMainWorld(
    "skipIntroAnimation",
    isDev || skipIntroAnimation
  )
}

contextBridge.exposeInMainWorld(
  "onShowWindowCloseModal",
  (cb: (reasons: CloseWarningReasons) => void) =>
    subscribeCloseWarning(ipcRenderer, cb)
)
// What a restart or an update install would stop, for the question asked
// before either; nothing when the close warning is switched off.
contextBridge.exposeInMainWorld("shutdownStops", async () =>
  ipcRenderer.invoke("shutdownStops")
)

contextBridge.exposeInMainWorld("closeWindow", async () =>
  ipcRenderer.invoke("closeWindow")
)

// The core stopped by itself: the page says so and offers a restart. The
// second is for a page that loads after it happened.
contextBridge.exposeInMainWorld(
  "onCoreExited",
  (cb: (exit: CoreExit) => void) => subscribeCoreExit(ipcRenderer, cb)
)
contextBridge.exposeInMainWorld("coreExit", async () =>
  coreExit(await ipcRenderer.invoke("coreExit"))
)

// The prompt shown when a quit, restart or update does not finish.
contextBridge.exposeInMainWorld(
  "onShutdownPrompt",
  (cb: (message: unknown) => void) => subscribeShutdownPrompt(ipcRenderer, cb)
)
contextBridge.exposeInMainWorld(
  "answerShutdownPrompt",
  (id: number, choice: string) =>
    ipcRenderer.send("shutdown-prompt-answer", { id, choice })
)
// The prompt is on screen; without this main asks with a native dialog.
contextBridge.exposeInMainWorld("shownShutdownPrompt", (id: number) =>
  ipcRenderer.send("shutdown-prompt-shown", id)
)
// The prompt's text in the user's language, for the native dialog main
// shows when no App can (a startup error screen after a crash, a reload).
contextBridge.exposeInMainWorld(
  "setShutdownPromptText",
  (text: Record<string, string>) =>
    ipcRenderer.send("shutdown-prompt-text", text)
)

// Protocol URL handler (curseforge://, modrinth://, gdlauncher://)
contextBridge.exposeInMainWorld("onProtocolUrl", (cb: (url: string) => void) =>
  subscribeProtocol(ipcRenderer, cb)
)
