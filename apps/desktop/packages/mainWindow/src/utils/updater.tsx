import { FEReleaseChannel } from "@gd/core_module/bindings"
import { createEffect, createSignal, onCleanup } from "solid-js"
import { rspc } from "./rspcClient"
import { toast, Progress } from "@gd/ui"

const TOAST_ID_CHECKING = "updater-checking"
const TOAST_ID_DOWNLOADING = "updater-downloading"
const TOAST_ID_MANUAL_UPDATE = "updater-manual-update"

export const [supportsAutoUpdate, setSupportsAutoUpdate] = createSignal(false)
export const [isManualCheck, setIsManualCheck] = createSignal(false)
export const [isCheckingForUpdates, setIsCheckingForUpdates] =
  createSignal(false)

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
      break

    case "no-update": {
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
      toast.dismiss(TOAST_ID_CHECKING)

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
      hadError = false
      break

    case "downloading": {
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
      toast.dismiss(TOAST_ID_CHECKING)

      const version = stateData.updateInfo?.version || "..."
      const percent = Math.round(stateData.progress)

      if (supportsAutoUpdate()) {
        toast.loading(`Downloading update v${version}`, {
          id: TOAST_ID_DOWNLOADING,
          description: (
            <div class="flex flex-col gap-2 w-full">
              <div class="flex items-center justify-between text-sm">
                <span>{`${percent}%`}</span>
                <span class="text-lightSlate-400">{`${percent}/100`}</span>
              </div>
              <Progress value={percent} size="small" />
            </div>
          ),
          duration: Infinity
        })
      } else if (stateData.updateInfo) {
        // For builds without auto-update, show manual download link
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
      toast.dismiss(TOAST_ID_DOWNLOADING)
      setIsCheckingForUpdates(false)

      if (hadError) {
        setIsManualCheck(false)
        return
      }

      if (supportsAutoUpdate()) {
        const ver = stateData.updateInfo?.version || "..."

        toast.success(`Update v${ver} ready to install`, {
          id: TOAST_ID_DOWNLOADING,
          description: "Install now or wait until you quit the app",
          duration: Infinity,
          onDismiss: () => {
            setIsManualCheck(false)
          },
          action: {
            label: "Install Now",
            onClick: () => {
              setIsManualCheck(false)
              window.installUpdate()
            }
          },
          cancel: {
            label: "Later",
            onClick: () => {
              setIsManualCheck(false)
            }
          }
        })
      }
      break

    case "error":
      clearManualCheckTimeout()
      setIsCheckingForUpdates(false)
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
      toast.success(`Update v${ver} ready to install`, {
        id: TOAST_ID_DOWNLOADING,
        description: "Install now or wait until you quit the app",
        duration: Infinity,
        action: {
          label: "Install Now",
          onClick: () => window.installUpdate()
        },
        cancel: {
          label: "Later",
          onClick: () => {}
        }
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
