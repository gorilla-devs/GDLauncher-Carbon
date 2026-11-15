import type { IpcRenderer } from "electron"
import type { UpdateStateData } from "../main/utils/updateState"
import { installReadyUpdateAction } from "./installReadyUpdate"

type RecoveryUpdateState =
  | "idle"
  | "checking"
  | "downloading"
  | "ready"
  | "error"
  | "no-update"
  | "unsupported"
  | "manual-update"

/**
 * The update check of the screens shown when the launcher cannot start: one
 * button that checks, shows the download, then applies it, with a status line
 * and a progress bar. It follows main's update state, and checks at once when
 * nothing has been checked yet.
 *
 * The core is down on these screens, so main picks the channel: the one last
 * checked, else this build's own, and never an older release.
 */
export async function recoveryUpdateCheck(io: {
  ipc: Pick<IpcRenderer, "invoke" | "on">
  button: HTMLButtonElement
  status: HTMLElement
  progress: HTMLElement
  bar: HTMLElement
  /** The status line before anything was checked. */
  idle: string
  /** Fills the status line when the channel has nothing newer. */
  noUpdate: (status: HTMLElement) => void
  /** Whether the status sits on one line beside the button. */
  oneLine: boolean
}) {
  const { button, status, progress, bar } = io
  let updateInfo: any = null
  let readyDownloadId: number | undefined
  let updateState: RecoveryUpdateState = "idle"
  const line = (value: "nowrap" | "normal") => {
    status.style.whiteSpace = io.oneLine ? value : "normal"
  }

  const setUpdateState = (state: RecoveryUpdateState, data?: any) => {
    updateState = state

    switch (state) {
      case "idle":
        status.textContent = io.idle
        line("nowrap")
        progress.style.display = "none"
        button.textContent = "Check for Updates"
        button.disabled = false
        button.style.opacity = "1"
        button.style.background = "rgb(var(--primary-500))"
        break

      case "checking":
        status.textContent = "Checking for updates..."
        line("nowrap")
        progress.style.display = "none"
        button.textContent = "Checking..."
        button.disabled = true
        button.style.opacity = "0.6"
        break

      case "manual-update":
        updateInfo = data
        status.textContent = `Update v${data?.version || "..."} available. Copy the download link to update manually.`
        progress.style.display = "none"
        button.textContent = "Copy Link"
        button.disabled = false
        button.style.opacity = "1"
        break

      case "downloading":
        if (data) {
          updateInfo = data
        }
        status.textContent = `Downloading v${updateInfo?.version || "..."}`
        line("nowrap")
        progress.style.display = "block"
        button.textContent = "Downloading..."
        button.disabled = true
        button.style.opacity = "0.6"
        break

      case "ready":
        status.textContent = `Update v${updateInfo?.version || "..."} ready`
        line("nowrap")
        progress.style.display = "none"
        button.textContent = "Apply & Restart"
        button.disabled = false
        button.style.opacity = "1"
        button.style.background = "rgb(var(--green-500))"
        break

      case "no-update":
        io.noUpdate(status)
        line("normal")
        progress.style.display = "none"
        button.textContent = "Check for Updates"
        button.disabled = false
        button.style.opacity = "1"
        button.style.background = "rgb(var(--primary-500))"
        break

      case "unsupported":
      case "error":
        status.textContent = data?.message || "Update check failed"
        line("nowrap")
        progress.style.display = "none"
        button.textContent = "Retry"
        button.disabled = false
        button.style.opacity = "1"
        button.style.background = "rgb(var(--primary-500))"
        break
    }
  }

  let lastUpdateRevision = -1
  const receiveUpdateState = (stateData: UpdateStateData) => {
    if (
      !Number.isSafeInteger(stateData.revision) ||
      stateData.revision <= lastUpdateRevision
    )
      return
    lastUpdateRevision = stateData.revision
    readyDownloadId = stateData.readyUpdate?.id
    if (
      stateData.readyUpdate &&
      !["error", "unsupported"].includes(stateData.state)
    ) {
      updateInfo = { version: stateData.readyUpdate.version }
      setUpdateState("ready")
      return
    }
    if (
      !["downloading", "downloaded", "manual-update"].includes(stateData.state)
    ) {
      updateInfo = null
      bar.style.width = "0%"
    }
    switch (stateData.state) {
      case "downloaded":
        updateInfo = stateData.updateInfo
        setUpdateState("ready")
        break
      case "downloading":
        updateInfo = stateData.updateInfo
        setUpdateState("downloading", updateInfo)
        bar.style.width = `${Math.round(stateData.progress || 0)}%`
        break
      case "manual-update":
        setUpdateState("manual-update", stateData.updateInfo)
        break
      case "error":
      case "unsupported":
        setUpdateState(stateData.state, stateData.error)
        break
      default:
        setUpdateState(stateData.state)
        break
    }
  }

  const checkForUpdates = async () => {
    setUpdateState("checking")

    try {
      const result = await io.ipc.invoke("checkForUpdatesFromRecovery")
      if (result?.status === "snapshot-version")
        setUpdateState("unsupported", {
          message: "Snapshot builds do not support update checks."
        })
      else if (result?.currentState) receiveUpdateState(result.currentState)
    } catch (error) {
      console.error("Update check failed:", error)
      setUpdateState("error", { message: "Failed to check for updates" })
    }
  }

  const installReady = installReadyUpdateAction(
    () => readyDownloadId,
    (id) => io.ipc.invoke("installUpdate", id),
    (message) => setUpdateState("error", { message })
  )
  button.addEventListener("click", async () => {
    if (updateState === "error" && readyDownloadId !== undefined) {
      await installReady()
      return
    }
    if (
      updateState === "idle" ||
      updateState === "error" ||
      updateState === "no-update" ||
      updateState === "unsupported"
    ) {
      await checkForUpdates()
    } else if (updateState === "manual-update" && updateInfo?.downloadUrl) {
      await navigator.clipboard.writeText(updateInfo.downloadUrl)
    } else if (updateState === "ready") {
      await installReady()
    }
  })

  io.ipc.on("gd-update-state-changed", (_, stateData) =>
    receiveUpdateState(stateData)
  )
  const currentState = await io.ipc.invoke("getUpdateState")
  if (currentState?.state === "idle") await checkForUpdates()
  else if (currentState) receiveUpdateState(currentState)
}
