import { FEReleaseChannel } from "@gd/core_module/bindings"
import { createEffect, createSignal, onCleanup } from "solid-js"
import { rspc } from "./rspcClient"
import { toast, Progress, Button } from "@gd/ui"

const TOAST_ID_CHECKING = "updater-checking"
const TOAST_ID_DOWNLOADING = "updater-downloading"
const TOAST_ID_MANUAL_UPDATE = "updater-manual-update"

export const [supportsAutoUpdate, setSupportsAutoUpdate] = createSignal(false)
export const [isManualCheck, setIsManualCheck] = createSignal(false)
export const [isCheckingForUpdates, setIsCheckingForUpdates] =
  createSignal(false)
export const [downloadProgress, setDownloadProgress] = createSignal(0)
const [isInstallingManually, setIsInstallingManually] = createSignal(false)

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

function UpdateReadyActions() {
  return (
    <div class="flex flex-col gap-2 w-full">
      <span class="text-sm text-lightSlate-400">
        Install now or wait until you quit the app
      </span>
      <div class="flex gap-2">
        <Button
          size="small"
          loading={isInstallingManually()}
          disabled={isInstallingManually()}
          onClick={() => {
            setIsInstallingManually(true)
            setIsManualCheck(false)
            window.installUpdate()
          }}
        >
          Install Now
        </Button>
        <Button
          size="small"
          type="secondary"
          disabled={isInstallingManually()}
          onClick={() => {
            setIsManualCheck(false)
            toast.dismiss(TOAST_ID_DOWNLOADING)
          }}
        >
          Later
        </Button>
      </div>
    </div>
  )
}

let lastChannel: FEReleaseChannel | null = null
let updateCheckInterval: ReturnType<typeof setInterval> | null = null
let manualCheckTimeout: ReturnType<typeof setTimeout> | null = null
let hadError = false

function clearManualCheckTimeout() {
  if (manualCheckTimeout) {
    clearTimeout(manualCheckTimeout)
    manualCheckTimeout = null
  }
}

;(async () => {
  const os = await window.getCurrentOS()
  setSupportsAutoUpdate(os.supportsAutoUpdate)
})()

window.onUpdateStateChanged((_, stateData) => {
  switch (stateData.state) {
    case "idle":
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      break

    case "no-update": {
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false

      const wasManualCheck = isManualCheck()
      setIsManualCheck(false)

      if (wasManualCheck) {
        toast.success("You're up to date!", {
          id: TOAST_ID_CHECKING,
          description: `GDLauncher ${__APP_VERSION__} is the latest version`,
          duration: 5000
        })
      }
      break
    }

    case "checking":
      setIsCheckingForUpdates(true)
      setDownloadProgress(0)
      hadError = false
      break

    case "downloading": {
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
      toast.dismiss(TOAST_ID_CHECKING)

      const version = stateData.updateInfo?.version || "..."
      setDownloadProgress(stateData.progress)

      if (supportsAutoUpdate()) {
        // Only create the toast once, let the signal update the UI
        if (!isShowingDownloadToast) {
          isShowingDownloadToast = true
          toast.loading(`Downloading update v${version}`, {
            id: TOAST_ID_DOWNLOADING,
            description: <DownloadProgress />,
            duration: Infinity
          })
        }
      } else if (stateData.updateInfo && !isShowingDownloadToast) {
        // For builds without auto-update, show manual download link
        isShowingDownloadToast = true
        toast(`Update available: v${version}`, {
          id: TOAST_ID_MANUAL_UPDATE,
          description:
            "Auto-update not available. Download manually to update.",
          duration: Infinity,
          action: {
            label: "Copy Download Link",
            onClick: async () => {
              try {
                await navigator.clipboard.writeText(
                  stateData.updateInfo!.downloadUrl
                )
                toast.success("Download link copied to clipboard", {
                  duration: 3000
                })
              } catch (error) {
                console.error("Failed to copy to clipboard:", error)
                toast.error("Failed to copy link", {
                  description: "Please try copying manually",
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
      clearManualCheckTimeout()
      toast.dismiss(TOAST_ID_CHECKING)
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      setIsInstallingManually(false)

      if (hadError) {
        setIsManualCheck(false)
        return
      }

      if (supportsAutoUpdate()) {
        const ver = stateData.updateInfo?.version || "..."

        toast.success(`Update v${ver} ready to install`, {
          id: TOAST_ID_DOWNLOADING,
          description: <UpdateReadyActions />,
          duration: Infinity,
          onDismiss: () => {
            setIsManualCheck(false)
          }
        })
      }
      break

    case "error":
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
      isShowingDownloadToast = false
      hadError = true
      setIsManualCheck(false)

      toast.error("Update failed", {
        id: TOAST_ID_CHECKING,
        description:
          stateData.error?.message ||
          "An error occurred during the update process",
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
      setIsInstallingManually(false)
      toast.success(`Update v${ver} ready to install`, {
        id: TOAST_ID_DOWNLOADING,
        description: <UpdateReadyActions />,
        duration: Infinity
      })
      return
    }
  } catch (error) {
    console.error("Failed to get update state:", error)
  }

  setIsManualCheck(true)
  setIsCheckingForUpdates(true)

  toast.loading("Checking for updates...", {
    id: TOAST_ID_CHECKING,
    description: "Looking for new versions",
    duration: Infinity
  })

  // Clear any existing timeout and set a new one
  clearManualCheckTimeout()
  manualCheckTimeout = setTimeout(() => {
    setIsManualCheck(false)
    setIsCheckingForUpdates(false)
    toast.error("Update check timed out", {
      id: TOAST_ID_CHECKING,
      description:
        "Unable to communicate with update service. Please try again.",
      duration: 5000
    })
  }, 35000)

  try {
    const result = await window.checkForUpdates(releaseChannel)

    if (result?.status === "busy") {
      clearManualCheckTimeout()
      setIsManualCheck(false)
      setIsCheckingForUpdates(false)
      toast.info("Update in progress", {
        id: TOAST_ID_CHECKING,
        description: "An update operation is already in progress",
        duration: 5000
      })
      return
    }

    if (result?.status === "snapshot-version") {
      clearManualCheckTimeout()
      setIsManualCheck(false)
      setIsCheckingForUpdates(false)
      toast.info("Snapshot version", {
        id: TOAST_ID_CHECKING,
        description: "Update checks are disabled for snapshot builds",
        duration: 5000
      })
      return
    }
    // Note: timeout will be cleared by state change handlers
  } catch (error) {
    clearManualCheckTimeout()
    console.error("Failed to trigger update check:", error)
    setIsManualCheck(false)
    setIsCheckingForUpdates(false)

    toast.error("Update check failed", {
      id: TOAST_ID_CHECKING,
      description: "Could not start update check. Please try again.",
      duration: 5000
    })
  }
}

export default checkForUpdates
