import { FEReleaseChannel } from "@gd/core_module/bindings"
import { createEffect, createMemo, createSignal, onCleanup } from "solid-js"
import { rspc } from "./rspcClient"
import { toast, Progress } from "@gd/ui"
import { i18n } from "@gd/i18n"
import type {
  UpdateErrorCode,
  UpdateStateData
} from "../../../main/utils/updateState"

const TOAST_ID_CHECKING = "updater-checking"
const TOAST_ID_DOWNLOADING = "updater-downloading"
const TOAST_ID_MANUAL_UPDATE = "updater-manual-update"

export const [supportsAutoUpdate, setSupportsAutoUpdate] = createSignal(false)
export const [isManualCheck, setIsManualCheck] = createSignal(false)
export const [isCheckingForUpdates, setIsCheckingForUpdates] =
  createSignal(false)
export const [downloadProgress, setDownloadProgress] = createSignal(0)
export const [hasPendingUpdate, setHasPendingUpdate] = createSignal(false)
export const [pendingUpdateVersion, setPendingUpdateVersion] = createSignal<
  string | null
>(null)

// Store downloading version in a signal for reactive access
const [downloadingVersion, setDownloadingVersion] = createSignal("...")

// Language signal for reactive translations - updates when i18next language changes
const [currentLanguage, setCurrentLanguage] = createSignal(
  i18n.language || "english"
)

// Subscribe to i18next language changes (runs once at module load)
i18n.on("languageChanged", (lang: string) => {
  setCurrentLanguage(lang)
})

// Reactive translation function - accesses language signal to trigger re-renders
function t(key: string, options?: Record<string, unknown>) {
  currentLanguage() // Access signal to create reactive dependency
  return i18n.t(key, options)
}

// Reactive toast components that update when language changes
function UpdateReadyTitle() {
  return (
    <>
      {t("app:_trn_update_ready", { version: pendingUpdateVersion() || "..." })}
    </>
  )
}

function UpdateReadyDescription() {
  return <>{t("app:_trn_update_ready_description")}</>
}

function UpdateInstallingTitle() {
  return <>{t("app:_trn_update_installing")}</>
}

function UpdateInstallingDescription() {
  return <>{t("app:_trn_update_installing_description")}</>
}

function InstallButtonLabel() {
  return <>{t("app:_trn_update_install_button")}</>
}

function LaterButtonLabel() {
  return <>{t("app:_trn_update_later_button")}</>
}

function DownloadingTitle() {
  return (
    <>{t("app:_trn_update_downloading", { version: downloadingVersion() })}</>
  )
}

function ManualUpdateTitle() {
  return <>{t("app:_trn_update_version", { version: downloadingVersion() })}</>
}

function ManualUpdateDescription() {
  return <>{t("app:_trn_update_manual_required")}</>
}

function CopyLinkButtonLabel() {
  return <>{t("app:_trn_update_copy_link")}</>
}

function CheckingTitle() {
  return <>{t("app:_trn_update_checking")}</>
}

function CheckingDescription() {
  return <>{t("app:_trn_update_checking_description")}</>
}

// Main describes each failure in English for its logs and tags it with a code;
// players read the translated text for that code.
const UPDATE_ERROR_TEXT: Record<UpdateErrorCode, string> = {
  "prepare-failed": "app:_trn_update_prepare_failed_description",
  "download-unavailable": "app:_trn_update_download_unavailable_description",
  "update-changed": "app:_trn_update_changed_description",
  "update-failed": "app:_trn_update_connection_failed_description",
  "check-failed": "app:_trn_update_check_error_description",
  "no-manual-download": "app:_trn_update_unavailable_no_download_description",
  "manual-check-failed": "app:_trn_update_unavailable_check_failed_description",
  "install-folder-read-only":
    "app:_trn_update_install_folder_read_only_description"
}
const updateErrorText = (
  error: UpdateStateData["error"] | undefined,
  fallbackKey: string
) => t((error?.code && UPDATE_ERROR_TEXT[error.code]) || fallbackKey)

let pendingReady: { id: number; version: string } | undefined
let lastReleaseChannel: FEReleaseChannel = "stable"
let installingReady: number | undefined
let lastUpdatePhase = "idle"
interface InstallPresentation {
  id: number
  retry: () => Promise<void>
  /** A failure toast already explains this attempt. */
  reported?: boolean
}
let installingPresentation: InstallPresentation | undefined
function installationFailure(message: string, captured: InstallPresentation) {
  if (
    (pendingReady && pendingReady.id !== captured.id) ||
    ["downloading", "manual-update"].includes(lastUpdatePhase)
  )
    return
  if (installingPresentation && installingPresentation !== captured) return
  if (installingPresentation === captured) installingPresentation = undefined
  captured.reported = true
  toast.error(t("app:_trn_update_failed"), {
    id: TOAST_ID_DOWNLOADING,
    description: message,
    duration: 8000,
    closeButton: true,
    cancel: undefined,
    action:
      pendingReady?.id === captured.id
        ? { label: <InstallButtonLabel />, onClick: captured.retry }
        : {
            label: i18n.t("general:_trn_retry"),
            onClick: () => manualCheckForUpdates(lastReleaseChannel)
          }
  })
}
export function showPendingUpdateToast() {
  const captured = pendingReady
  if (!captured || installingPresentation?.id === captured.id) return
  const install = async () => {
    if (pendingReady?.id !== captured.id || installingReady !== undefined)
      return
    installingReady = captured.id
    const presentation: InstallPresentation = {
      id: captured.id,
      retry: install
    }
    installingPresentation = presentation
    toast.loading(<UpdateInstallingTitle />, {
      id: TOAST_ID_DOWNLOADING,
      description: <UpdateInstallingDescription />,
      duration: Infinity,
      closeButton: false,
      action: undefined,
      cancel: undefined
    })
    try {
      await window.installUpdate(captured.id)
    } catch (error) {
      // The rejection carries main's English reason inside Electron's IPC
      // wrapper. When main also published the failure, its translated reason
      // is already on screen; otherwise the player gets the general one.
      console.error("Update installation failed:", error)
      if (!presentation.reported)
        installationFailure(
          t("app:_trn_update_install_failed_description"),
          presentation
        )
    } finally {
      if (installingReady === captured.id) installingReady = undefined
    }
  }
  installingPresentation = undefined
  toast.success(<UpdateReadyTitle />, {
    id: TOAST_ID_DOWNLOADING,
    description: <UpdateReadyDescription />,
    duration: Infinity,
    closeButton: true,
    action: {
      label: <InstallButtonLabel />,
      onClick: install
    },
    cancel: {
      label: <LaterButtonLabel />,
      onClick: () => {}
    }
  })
}

let isShowingDownloadToast = false

async function restartAfterFailedInstall() {
  try {
    await window.relaunch()
  } catch (error) {
    console.error("Restart after a failed update install failed:", error)
    toast.error(i18n.t("settings:_trn_restart_failed"))
  }
}
/** The core was stopped for an install that did not go through and nothing
 *  starts it again, so the launcher cannot be used until it restarts. */
function showRestartRequired() {
  toast.error(i18n.t("app:_trn_update_failed"), {
    id: TOAST_ID_CHECKING,
    description: t("app:_trn_update_restart_required_description"),
    duration: Infinity,
    closeButton: true,
    action: {
      label: i18n.t("settings:_trn_restart_app"),
      onClick: restartAfterFailedInstall
    }
  })
}

function DownloadProgress() {
  return (
    <div class="flex flex-col gap-2 w-full">
      <div class="flex items-center justify-between text-sm">
        <span>{`${Math.round(downloadProgress())}%`}</span>
      </div>
      <Progress value={downloadProgress()} size="small" />
    </div>
  )
}

const updateCapability = (async () => {
  try {
    const os = await window.getCurrentOS()
    setSupportsAutoUpdate(os.supportsAutoUpdate)
  } catch {
    setSupportsAutoUpdate(false)
  }
})()

let lastUpdateRevision = -1
const receiveUpdateState = (stateData: UpdateStateData) => {
  if (
    !Number.isSafeInteger(stateData.revision) ||
    stateData.revision <= lastUpdateRevision
  )
    return
  lastUpdateRevision = stateData.revision
  lastUpdatePhase = stateData.state
  const previousReadyId = pendingReady?.id
  pendingReady = stateData.readyUpdate ?? undefined
  const keepReady = !!pendingReady && supportsAutoUpdate()
  if (keepReady) {
    setHasPendingUpdate(true)
    setPendingUpdateVersion(pendingReady!.version)
    // Hydration can arrive after a later check has changed the phase. Publish
    // only newly observed authority, preserving dismissal and current attempts.
    if (
      previousReadyId !== pendingReady!.id &&
      stateData.state !== "downloaded"
    )
      showPendingUpdateToast()
  }
  if (
    !keepReady &&
    !["downloading", "downloaded", "manual-update"].includes(stateData.state)
  ) {
    setHasPendingUpdate(false)
    setPendingUpdateVersion(null)
    setDownloadingVersion("...")
    setDownloadProgress(0)
    toast.dismiss(TOAST_ID_DOWNLOADING)
    toast.dismiss(TOAST_ID_MANUAL_UPDATE)
  }
  switch (stateData.state) {
    case "idle":
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      break

    case "no-update": {
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false

      const wasManualCheck = isManualCheck()
      setIsManualCheck(false)

      if (wasManualCheck) {
        toast.success(i18n.t("app:_trn_update_up_to_date"), {
          id: TOAST_ID_CHECKING,
          description: i18n.t("app:_trn_update_latest_version", {
            version: __APP_VERSION__
          }),
          duration: 5000
        })
      }
      break
    }

    case "checking":
      setIsCheckingForUpdates(true)
      setDownloadProgress(0)
      break

    case "manual-update":
    case "downloading": {
      if (keepReady) break
      installingPresentation = undefined
      setHasPendingUpdate(false)
      setPendingUpdateVersion(null)
      setIsCheckingForUpdates(false)
      // A restart notice stays: a later download does not bring the core back.
      if (!stateData.restartRequired) toast.dismiss(TOAST_ID_CHECKING)
      setIsManualCheck(false)

      const version = stateData.updateInfo?.version || "..."
      setDownloadingVersion(version)
      setDownloadProgress(stateData.progress)

      if (stateData.state === "downloading" && supportsAutoUpdate()) {
        // Add class to suppress animation on updates (not initial creation)
        const isUpdate = isShowingDownloadToast
        isShowingDownloadToast = true

        // Re-create toast on every progress update to show current progress
        // (somoto doesn't maintain SolidJS reactivity for already-rendered content)
        toast.loading(<DownloadingTitle />, {
          id: TOAST_ID_DOWNLOADING,
          description: <DownloadProgress />,
          duration: Infinity,
          closeButton: false,
          action: undefined,
          cancel: undefined,
          className: isUpdate ? "toast-no-animate" : undefined
        })
      } else if (stateData.updateInfo?.downloadUrl && !isShowingDownloadToast) {
        // For builds without auto-update, show manual download link
        isShowingDownloadToast = true
        toast(<ManualUpdateTitle />, {
          id: TOAST_ID_MANUAL_UPDATE,
          description: <ManualUpdateDescription />,
          duration: Infinity,
          action: {
            label: <CopyLinkButtonLabel />,
            onClick: async () => {
              try {
                await navigator.clipboard.writeText(
                  stateData.updateInfo!.downloadUrl!
                )
                toast.success(i18n.t("app:_trn_update_link_copied"), {
                  duration: 3000
                })
              } catch (error) {
                console.error("Failed to copy to clipboard:", error)
                toast.error(i18n.t("app:_trn_update_copy_failed"), {
                  description: i18n.t(
                    "app:_trn_update_copy_failed_description"
                  ),
                  duration: 3000
                })
              }
            }
          }
        })
      }
      break
    }

    case "downloaded":
      if (!stateData.restartRequired) toast.dismiss(TOAST_ID_CHECKING)
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false

      if (keepReady) {
        const ver = pendingReady!.version
        setHasPendingUpdate(true)
        setPendingUpdateVersion(ver)
        showPendingUpdateToast()
      }
      break

    case "unsupported": {
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      const wasManualCheck = isManualCheck()
      setIsManualCheck(false)
      // This copy cannot update itself (a package manager or an unpacked
      // build owns it). Automatic checks at launch and every 30 minutes stay
      // quiet; a check the player asked for says why and where to get the
      // latest version.
      if (!wasManualCheck) break
      const version = stateData.updateInfo?.version
      toast.info(
        version
          ? i18n.t("app:_trn_update_version", { version })
          : i18n.t("app:_trn_update_check_failed"),
        {
          id: TOAST_ID_CHECKING,
          description: updateErrorText(
            stateData.error,
            "app:_trn_update_unavailable_check_failed_description"
          ),
          duration: 8000,
          closeButton: true,
          // Only a check that could not finish is worth repeating.
          action: version
            ? undefined
            : {
                label: i18n.t("general:_trn_retry"),
                onClick: () => manualCheckForUpdates(lastReleaseChannel)
              }
        }
      )
      break
    }

    case "error": {
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      setIsManualCheck(false)
      // An installer that fails to start (antivirus, a locked file) reports
      // the same code as a failed download, but the update is already on
      // disk, so the player is not sent to check their connection.
      const installerFailed =
        !!installingPresentation && stateData.error?.code === "update-failed"
      if (installingPresentation)
        installationFailure(
          installerFailed
            ? t("app:_trn_update_install_failed_description")
            : updateErrorText(
                stateData.error,
                "app:_trn_update_install_failed_description"
              ),
          installingPresentation
        )
      else if (previousReadyId !== undefined && !keepReady)
        installationFailure(
          updateErrorText(
            stateData.error,
            "app:_trn_update_prepare_failed_description"
          ),
          {
            id: previousReadyId,
            retry: () => manualCheckForUpdates(lastReleaseChannel)
          }
        )

      if (stateData.restartRequired) {
        showRestartRequired()
        break
      }
      toast.error(i18n.t("app:_trn_update_failed"), {
        id: TOAST_ID_CHECKING,
        description: installerFailed
          ? t("app:_trn_update_install_failed_description")
          : updateErrorText(
              stateData.error,
              "app:_trn_update_failed_description"
            ),
        duration: 8000,
        closeButton: true,
        action: !keepReady
          ? {
              label: i18n.t("general:_trn_retry"),
              onClick: () => manualCheckForUpdates(lastReleaseChannel)
            }
          : undefined
      })
      break
    }
  }
}
window.onUpdateStateChanged((_, stateData) => {
  void updateCapability.then(() => receiveUpdateState(stateData))
})
void updateCapability
  .then(() => window.getUpdateState())
  .then((state) => {
    if (state) receiveUpdateState(state)
  })
  .catch(() => {})

export const checkForUpdates = async () => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const releaseChannel = createMemo(() => settings.data?.releaseChannel)
  createEffect(() => {
    const channel = releaseChannel()
    if (!channel) return
    lastReleaseChannel = channel

    try {
      window.checkForUpdates(channel)
    } catch (error) {
      console.error("Failed to trigger automatic update check:", error)
    }

    const updateCheckInterval = setInterval(
      () => {
        try {
          window.checkForUpdates(channel)
        } catch (error) {
          console.error("Failed to trigger periodic update check:", error)
        }
      },
      60 * 30 * 1000
    )
    onCleanup(() => {
      clearInterval(updateCheckInterval)
    })
  })
}

export const manualCheckForUpdates = async (
  releaseChannel: FEReleaseChannel
) => {
  lastReleaseChannel = releaseChannel
  if (isCheckingForUpdates()) {
    return
  }

  try {
    const currentState = await window.getUpdateState()
    // A check cannot bring the stopped core back; the notice says what does.
    if (currentState?.restartRequired) showRestartRequired()
    if (currentState?.readyUpdate && supportsAutoUpdate()) {
      receiveUpdateState(currentState)
      showPendingUpdateToast()
      return
    }
    if (currentState?.restartRequired) return
  } catch (error) {
    console.error("Failed to get update state:", error)
  }

  setIsManualCheck(true)
  setIsCheckingForUpdates(true)

  toast.loading(<CheckingTitle />, {
    id: TOAST_ID_CHECKING,
    description: <CheckingDescription />,
    duration: Infinity
  })

  try {
    const result = await window.checkForUpdates(releaseChannel)

    if (
      result &&
      ["unsupported", "manual-update", "no-update", "error"].includes(
        result.status
      )
    ) {
      receiveUpdateState(result.currentState)
      return
    }

    if (result?.status === "busy") {
      setIsManualCheck(false)
      setIsCheckingForUpdates(false)
      toast.info(i18n.t("app:_trn_update_in_progress"), {
        id: TOAST_ID_CHECKING,
        description: i18n.t("app:_trn_update_in_progress_description"),
        duration: 5000
      })
      return
    }

    if (result?.status === "snapshot-version") {
      setIsManualCheck(false)
      setIsCheckingForUpdates(false)
      toast.info(i18n.t("app:_trn_update_snapshot"), {
        id: TOAST_ID_CHECKING,
        description: i18n.t("app:_trn_update_snapshot_description"),
        duration: 5000
      })
      return
    }
  } catch (error) {
    console.error("Failed to trigger update check:", error)
    setIsManualCheck(false)
    setIsCheckingForUpdates(false)

    toast.error(i18n.t("app:_trn_update_check_failed"), {
      id: TOAST_ID_CHECKING,
      description: i18n.t("app:_trn_update_check_failed_description"),
      duration: 5000
    })
  }
}

export default checkForUpdates
