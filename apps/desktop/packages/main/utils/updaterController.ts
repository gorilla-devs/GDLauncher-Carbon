import type { FEReleaseChannel } from "@gd/core_module/bindings"
import type { UpdateInfo } from "electron-updater"
import type { CheckForUpdatesResult, UpdateStateData } from "./updateState"

/** English reasons, for logs and the untranslated loading screen, why an
 *  install was refused before anything was stopped. */
const BLOCKED_INSTALL: Partial<Record<UpdateErrorCode, string>> = {
  "install-folder-read-only":
    "GDLauncher cannot replace its AppImage because it cannot write to the folder the AppImage is in. Move the AppImage to a folder you own and open it from there, or download the new version from gdlauncher.com."
}

export function createUpdaterController(io: {
  version: string
  updater: {
    channel: string | null
    allowPrerelease: boolean
    allowDowngrade: boolean
    checkForUpdates(): Promise<unknown>
    quitAndInstall(silent: boolean, run: boolean): void
    on(event: string, callback: (...args: any[]) => void): unknown
  }
  broadcast(state: UpdateStateData): void
  artifact(info: UpdateInfo): string | null
  /** Metadata for an installation electron-updater cannot update: no update
   *  when it is current, otherwise the newer version with its matching
   *  download, or `downloadUrl: null` when the channel has none for it. */
  manual(): Promise<
    (UpdateInfo & { downloadUrl: string | null }) | { noUpdate: true } | null
  >
  prepareShutdown(): Promise<void>
}) {
  let locked = false
  let revision = 0
  // Set once an install stopped the core. Nothing starts it again in this
  // process, so every later state says a restart is needed.
  let coreStopped = false
  let state: UpdateStateData = {
    revision: 0,
    state: "idle",
    updateInfo: null,
    progress: 0,
    error: null
  }
  function set(update: Partial<UpdateStateData>) {
    state = { ...state, ...update, revision: state.revision + 1 }
    io.broadcast(state)
  }
  function unavailable(message: string) {
    locked = false
    set({
      state: "unsupported",
      updateInfo,
      progress: 0,
      error: { message, details: "" }
    })
  }
  io.updater.on("update-available", (info: UpdateInfo) => {
    // Automatic download still owns the lock. URL resolution reads the exact
    // active provider metadata and never starts another check.
    let downloadUrl: string | null = null
    try {
      downloadUrl = io.artifact(info)
    } catch {
      /* Native auto-download remains independently supported. */
    }
    set({
      state: "downloading",
      progress: 0,
      error: null,
      updateInfo: { ...info, downloadUrl }
    })
  })
  io.updater.on("update-not-available", () => {
    locked = false
    set({ state: "no-update", updateInfo: null, progress: 0, error: null })
  })
  io.updater.on("download-progress", (progress: { percent: number }) =>
    set({ progress: progress.percent })
  )
  io.updater.on("update-downloaded", () => {
    locked = false
    set({ state: "downloaded", progress: 100 })
  })
  io.updater.on("error", () => {
    revision++
    locked = false
    set({
      state: "error",
      updateInfo: null,
      progress: 0,
      error: {
        message: "Update failed. Check your connection and retry.",
        details: ""
      }
    })
  })
  return {
    getState: () => state,
    async check(channel: FEReleaseChannel): Promise<CheckForUpdatesResult> {
      if (locked) return { status: "busy", currentState: state }
      if (io.version.includes("snapshot"))
        return { status: "snapshot-version", currentState: state }
      locked = true
      const ownedRevision = ++revision
      set({ state: "checking", updateInfo: null, error: null, progress: 0 })
      const numbers = { stable: 0, beta: 1, alpha: 2 }
      const current = io.version.includes("alpha")
        ? 2
        : io.version.includes("beta")
          ? 1
          : 0
      io.updater.channel = channel === "stable" ? "latest" : channel
      io.updater.allowPrerelease = channel !== "stable"
      io.updater.allowDowngrade = numbers[channel] < current
      try {
        const result = await io.updater.checkForUpdates()
        if (ownedRevision !== revision)
          return { status: "checking", currentState: state }
        if (result === null) {
          try {
            const metadata = await io.manual()
            if (ownedRevision !== revision)
              return { status: "checking", currentState: state }
            locked = false
            if (metadata && "noUpdate" in metadata)
              set({
                state: "no-update",
                updateInfo: null,
                progress: 0,
                error: null
              })
            else if (metadata)
              set({
                state: "manual-update",
                updateInfo: metadata,
                progress: 0,
                error: null
              })
            else
              unavailable(
                "Automatic updates are unavailable for this installation. No matching manual artifact is available; check the official downloads or retry later."
              )
          } catch {
            if (ownedRevision !== revision)
              return { status: "checking", currentState: state }
            unavailable(
              "Automatic updates are unavailable. Manual update metadata could not be checked; check the official downloads or retry."
            )
          }
        }
      } catch {
        if (ownedRevision !== revision)
          return { status: "checking", currentState: state }
        locked = false
        set({
          state: "error",
          updateInfo: null,
          progress: 0,
          error: {
            message: "Failed to check for updates. Please retry.",
            details: ""
          }
        })
      }
      return {
        status: ([
          "unsupported",
          "manual-update",
          "no-update",
          "error"
        ].includes(state.state)
          ? state.state
          : "checking") as CheckForUpdatesResult["status"],
        currentState: state
      }
    },
    async install() {
      if (state.state !== "downloaded")
        throw new Error("No automatic update is ready to install")
      await io.prepareShutdown()
      io.updater.quitAndInstall(true, true)
    }
  }
}
