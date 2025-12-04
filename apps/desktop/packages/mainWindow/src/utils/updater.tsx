import { FEReleaseChannel } from "@gd/core_module/bindings"
import { createEffect, createSignal, onCleanup } from "solid-js"
import { rspc } from "./rspcClient"
import { toast, Progress } from "@gd/ui"
import { i18n } from "@gd/i18n"

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
const [currentLanguage, setCurrentLanguage] = createSignal(i18n.language || "english")

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
  return <>{t("app:_trn_update_ready", { version: pendingUpdateVersion() || "..." })}</>
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
  return <>{t("app:_trn_update_downloading", { version: downloadingVersion() })}</>
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

export function showPendingUpdateToast() {
  toast.success(<UpdateReadyTitle />, {
    id: TOAST_ID_DOWNLOADING,
    description: <UpdateReadyDescription />,
    duration: Infinity,
    action: {
      label: <InstallButtonLabel />,
      onClick: () => {
        toast.loading(<UpdateInstallingTitle />, {
          id: TOAST_ID_DOWNLOADING,
          description: <UpdateInstallingDescription />,
          duration: Infinity,
          closeButton: false
        })
        window.installUpdate()
      }
    },
    cancel: {
      label: <LaterButtonLabel />,
      onClick: () => {}
    }
  })
}

let isShowingDownloadToast = false

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

let lastChannel: FEReleaseChannel | null = null
let updateCheckInterval: ReturnType<typeof setInterval> | null = null

;(async () => {
  const os = await window.getCurrentOS()
  setSupportsAutoUpdate(os.supportsAutoUpdate)
})()

window.onUpdateStateChanged((_, stateData) => {
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
          description: i18n.t("app:_trn_update_latest_version", { version: __APP_VERSION__ }),
          duration: 5000
        })
      }
      break
    }

    case "checking":
      setIsCheckingForUpdates(true)
      setDownloadProgress(0)
      break

    case "downloading": {
      setIsCheckingForUpdates(false)
      toast.dismiss(TOAST_ID_CHECKING)

      const version = stateData.updateInfo?.version || "..."
      setDownloadingVersion(version)
      setDownloadProgress(stateData.progress)

      if (supportsAutoUpdate()) {
        // Only create the toast once, let the signal update the UI
        if (!isShowingDownloadToast) {
          isShowingDownloadToast = true
          toast.loading(<DownloadingTitle />, {
            id: TOAST_ID_DOWNLOADING,
            description: <DownloadProgress />,
            duration: Infinity,
            closeButton: false
          })
        }
      } else if (stateData.updateInfo && !isShowingDownloadToast) {
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
                  stateData.updateInfo!.downloadUrl
                )
                toast.success(i18n.t("app:_trn_update_link_copied"), {
                  duration: 3000
                })
              } catch (error) {
                console.error("Failed to copy to clipboard:", error)
                toast.error(i18n.t("app:_trn_update_copy_failed"), {
                  description: i18n.t("app:_trn_update_copy_failed_description"),
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
      toast.dismiss(TOAST_ID_CHECKING)
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false

      if (supportsAutoUpdate()) {
        const ver = stateData.updateInfo?.version || "..."
        setHasPendingUpdate(true)
        setPendingUpdateVersion(ver)
        showPendingUpdateToast()
      }
      break

    case "error":
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      setIsManualCheck(false)

      toast.error(i18n.t("app:_trn_update_failed"), {
        id: TOAST_ID_CHECKING,
        description:
          stateData.error?.message ||
          i18n.t("app:_trn_update_failed_description"),
        duration: 8000
      })
      break
  }
})

export const checkForUpdates = async () => {
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  createEffect(() => {
    if (!settings.data) return

    if (!lastChannel || settings.data.releaseChannel !== lastChannel) {
      lastChannel = settings.data.releaseChannel

      if (updateCheckInterval) {
        clearInterval(updateCheckInterval)
        updateCheckInterval = null
      }

      try {
        window.checkForUpdates(lastChannel)
      } catch (error) {
        console.error("Failed to trigger automatic update check:", error)
      }

      updateCheckInterval = setInterval(
        () => {
          try {
            window.checkForUpdates(lastChannel!)
          } catch (error) {
            console.error("Failed to trigger periodic update check:", error)
          }
        },
        60 * 30 * 1000
      )
    }

    onCleanup(() => {
      if (updateCheckInterval) {
        clearInterval(updateCheckInterval)
        updateCheckInterval = null
      }
    })
  })
}

export const manualCheckForUpdates = async (
  releaseChannel: FEReleaseChannel
) => {
  if (isCheckingForUpdates()) {
    return
  }

  try {
    const currentState = await window.getUpdateState()
    if (currentState?.state === "downloaded" && supportsAutoUpdate()) {
      const ver = currentState.updateInfo?.version || "..."
      setPendingUpdateVersion(ver)
      showPendingUpdateToast()
      return
    }
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
