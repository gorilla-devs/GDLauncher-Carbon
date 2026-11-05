import type { FEReleaseChannel } from "@gd/core_module/bindings"
import type { UpdateInfo } from "electron-updater"
import type {
  CheckForUpdatesResult,
  UpdateErrorCode,
  UpdateStateData
} from "./updateState"
import {
  createOwnedNativePreparation,
  type OwnedNativePreparation
} from "./ownedNativePreparation"

interface InstallAttempt {
  stage: "settling" | "shutdown" | "dispatching" | "pending"
  failed: boolean
  retire?: () => void
}
interface DownloadOperation {
  info: UpdateInfo
  preparation?: Promise<void>
  native?: OwnedNativePreparation
}

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
  isDownloaded(info: UpdateInfo): boolean
  dispatchInstall?(): () => void
}) {
  let locked = false
  let installing: InstallAttempt | null = null
  const checkSettlements = new Set<Promise<void>>()
  const downloadSettlements = new Set<Promise<void>>()
  const downloadOperations = new WeakMap<object, DownloadOperation>()
  const nativePreparation = createOwnedNativePreparation(io.updater)
  let activeDownload: DownloadOperation | null = null
  let downloaded: {
    id: number
    info: UpdateInfo
    operation: DownloadOperation
  } | null = null
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
    state = {
      ...state,
      ...update,
      readyUpdate: downloaded
        ? { id: downloaded.id, version: downloaded.info.version }
        : null,
      ...(coreStopped ? { restartRequired: true as const } : {}),
      revision: state.revision + 1
    }
    io.broadcast(state)
  }
  const nativeInstallPending = () =>
    installing?.stage === "dispatching" || installing?.stage === "pending"
  function failPreparation(operation: DownloadOperation, error: unknown) {
    if (downloaded?.operation !== operation && activeDownload !== operation)
      return
    nativePreparation.rememberError(error)
    operation.native?.dispose()
    if (downloaded?.operation === operation) downloaded = null
    if (activeDownload === operation) activeDownload = null
    revision++
    locked = false
    const attempt = installing
    if (attempt) {
      attempt.failed = true
      if (attempt.stage === "pending") releaseFailedDispatch(attempt)
    }
    set({
      state: "error",
      updateInfo: null,
      progress: 0,
      error: {
        message: "Update preparation failed. Check for updates and retry.",
        details: "",
        code: "prepare-failed"
      }
    })
  }
  function retainDownloadSettlement(result: unknown) {
    const value = result as {
      updateInfo?: UpdateInfo
      versionInfo?: UpdateInfo
      downloadPromise?: Promise<unknown>
    } | null
    const work = value?.downloadPromise
    if (!work || typeof work.then !== "function") return
    // AppUpdater returns the exact object emitted by update-available. Capture
    // that operation, rather than infer ownership from an equal version/hash.
    const info = value?.updateInfo ?? value?.versionInfo
    const operation = info && downloadOperations.get(info)
    // The check settles before its automatic download. Mac's outer downloaded
    // event also precedes native serving completion, so this is not an install
    // admission gate. After failure it fences the known late error callbacks.
    const settlement = Promise.resolve(work).then(
      () => {},
      (error) => {
        if (!operation) return
        // Helper metadata precedes Mac native preparation and survives its
        // rejection. Retire that failed authority so recovery starts fresh work.
        failPreparation(operation, error)
      }
    )
    if (operation) operation.preparation = settlement
    downloadSettlements.add(settlement)
    void settlement.then(() => {
      downloadSettlements.delete(settlement)
      if (operation?.preparation === settlement)
        operation.preparation = undefined
    })
  }
  function releaseFailedDispatch(attempt: InstallAttempt) {
    attempt.retire?.()
    void Promise.all([...downloadSettlements]).then(() => {
      if (installing === attempt && attempt.failed) installing = null
    })
  }
  function unavailable(
    code: UpdateErrorCode,
    message: string,
    updateInfo: UpdateStateData["updateInfo"] = null
  ) {
    locked = false
    set({
      state: "unsupported",
      updateInfo,
      progress: 0,
      error: { message, details: "", code }
    })
  }
  io.updater.on("update-available", (info: UpdateInfo) => {
    if (nativeInstallPending()) return
    downloaded?.operation.native?.dispose()
    activeDownload?.native?.dispose()
    downloaded = null
    activeDownload = {
      info: { ...info, files: info.files.map((file) => ({ ...file })) }
    }
    downloadOperations.set(info, activeDownload)
    locked = true
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
    if (nativeInstallPending()) return
    locked = false
    set({ state: "no-update", updateInfo: null, progress: 0, error: null })
  })
  io.updater.on("download-progress", (progress: { percent: number }) => {
    if (!nativeInstallPending()) set({ progress: progress.percent })
  })
  io.updater.on("update-downloaded", () => {
    if (nativeInstallPending()) return
    locked = false
    if (!activeDownload || !io.isDownloaded(activeDownload.info)) {
      downloaded = null
      set({
        state: "error",
        error: {
          message:
            "The downloaded update is unavailable. Check for updates and retry.",
          details: "",
          code: "download-unavailable"
        }
      })
      return
    }
    const operation = activeDownload
    operation.native = nativePreparation.watch((error) =>
      failPreparation(operation, error)
    )
    downloaded = { id: state.revision + 1, info: operation.info, operation }
    activeDownload = null
    set({ state: "downloaded", progress: 100 })
  })
  io.updater.on("error", (error: unknown) => {
    if (nativePreparation.ownsError(error)) return
    const attempt = installing
    if (attempt) attempt.failed = true
    revision++
    locked = false
    activeDownload = null
    set({
      state: "error",
      updateInfo: null,
      progress: 0,
      error: {
        message: "Update failed. Check your connection and retry.",
        details: "",
        code: "update-failed"
      }
    })
    // Keep synchronous failure ownership until quitAndInstall returns. A later
    // error releases admission only after known native download callbacks end.
    if (attempt?.stage === "pending") releaseFailedDispatch(attempt)
  })
  return {
    getState: () => state,
    async check(channel: FEReleaseChannel): Promise<CheckForUpdatesResult> {
      if (
        locked ||
        installing ||
        downloaded?.operation.preparation ||
        activeDownload?.preparation ||
        downloaded?.operation.native?.pending() ||
        activeDownload?.native?.pending()
      )
        return { status: "busy", currentState: state }
      if (io.version.includes("snapshot"))
        return { status: "snapshot-version", currentState: state }
      locked = true
      let settled!: () => void
      const settlement = new Promise<void>((resolve) => {
        settled = resolve
      })
      checkSettlements.add(settlement)
      try {
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
          retainDownloadSettlement(result)
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
              else if (metadata?.downloadUrl)
                set({
                  state: "manual-update",
                  updateInfo: metadata,
                  progress: 0,
                  error: null
                })
              else
                unavailable(
                  "no-manual-download",
                  "Automatic updates are unavailable for this installation. No matching manual artifact is available; check the official downloads or retry later.",
                  metadata
                )
            } catch {
              if (ownedRevision !== revision)
                return { status: "checking", currentState: state }
              unavailable(
                "manual-check-failed",
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
              details: "",
              code: "check-failed"
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
      } finally {
        checkSettlements.delete(settlement)
        settled()
      }
    },
    async install(expectedId?: number) {
      const captured = downloaded
      if (!captured || (expectedId !== undefined && expectedId !== captured.id))
        throw new Error("No automatic update is ready to install")
      if (installing)
        throw new Error("Update installation is already in progress")
      const admitted = () =>
        downloaded === captured && io.isDownloaded(captured.info)
      const refuse = (code: UpdateErrorCode, message: string): never => {
        if (downloaded === captured) {
          captured.operation.native?.dispose()
          downloaded = null
          set({ state: "error", error: { message, details: "", code } })
        }
        throw new Error(message)
      }
      if (!admitted())
        refuse(
          "download-unavailable",
          "The downloaded update is unavailable. Check for updates and retry."
        )
      const attempt: InstallAttempt = { stage: "settling", failed: false }
      installing = attempt
      try {
        // Checks admitted before the click may still publish a replacement.
        // Reconcile all of them before shutdown; newly requested checks are busy.
        if (checkSettlements.size) await Promise.all([...checkSettlements])
        // Squirrel.Mac may still be fetching the update. If that failed after
        // the core was stopped, GDLauncher would be left without its core, so
        // wait for it while the core still runs.
        const native = captured.operation.native
        if (native?.pending()) await native.settled
        const blocker = io.installBlocker ? await io.installBlocker() : null
        if (!admitted())
          refuse(
            "update-changed",
            "The selected update changed while checking. Check for updates and retry."
          )
        if (attempt.failed)
          throw new Error("Update failed before installation. Please retry.")
        if (blocker) {
          // Nothing was stopped and the update stays ready: once the cause is
          // fixed, Install works again.
          const message =
            BLOCKED_INSTALL[blocker] ?? "The update cannot be installed here."
          set({
            state: "error",
            error: { message, details: "", code: blocker }
          })
          throw new Error(message)
        }
        attempt.stage = "shutdown"
        await io.prepareShutdown()
        if (!admitted())
          refuse(
            "update-changed",
            "The selected update changed during shutdown. Check for updates and retry."
          )
        if (attempt.failed)
          throw new Error("Update failed during shutdown. Please retry.")
        attempt.stage = "dispatching"
        if (io.dispatchInstall) attempt.retire = io.dispatchInstall()
        else io.updater.quitAndInstall(true, true)
        attempt.stage = "pending"
        // A void return is neither completion nor failure. Retain this owner
        // until a real error permits recovery or the application exits.
        if (attempt.failed) releaseFailedDispatch(attempt)
      } catch (error) {
        if (attempt.stage === "dispatching" || attempt.stage === "pending") {
          attempt.failed = true
          releaseFailedDispatch(attempt)
        } else if (installing === attempt) installing = null
        // A failure reported before the core finished stopping, or none at
        // all, still has to tell the player that only a restart helps now.
        if (coreStopped && !state.restartRequired)
          set({
            state: "error",
            progress: 0,
            error:
              state.state === "error" && state.error
                ? state.error
                : {
                    message:
                      "The update could not be installed. Restart GDLauncher to keep using it.",
                    details: "",
                    code: "update-failed"
                  }
          })
        throw error
      }
    }
  }
}
