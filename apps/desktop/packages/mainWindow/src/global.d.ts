import type { FEReleaseChannel } from "@gd/core_module/bindings"
import { BoundsSize } from "./utils/adhelper"
import type { UpdateInfo } from "electron-updater"
import type { Log } from "../../main/coreModule"

type UpdateState =
  | "idle"
  | "checking"
  | "downloading"
  | "downloaded"
  | "error"
  | "no-update"

interface UpdateStateData {
  state: UpdateState
  updateInfo: (UpdateInfo & { downloadUrl: string }) | null
  progress: number
  error: { message: string; details: string } | null
}

interface CheckForUpdatesResult {
  status: "checking" | "busy" | "snapshot-version"
  currentState: UpdateStateData
}

declare global {
  interface Window {
    fatalError: (error: string | Log[], moduleName?: string) => void
    backwardsMigrationError: () => void
    ipcRenderer: import("electron").IpcRenderer
    report: any
    getAdSize: () => Promise<{
      adSize: BoundsSize
      bannerAdSize?: BoundsSize
      hideAdText?: boolean
    }>
    openFileDialog: (
      filters: Electron.OpenDialogOptions
    ) => Promise<Electron.OpenDialogReturnValue>
    showSaveDialog: (
      filters: Electron.SaveDialogOptions
    ) => Promise<Electron.SaveDialogReturnValue>
    adSizeChanged: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void
    checkForUpdates: (
      releaseChannel: FEReleaseChannel
    ) => Promise<CheckForUpdatesResult | null>
    getUpdateState: () => Promise<UpdateStateData | null>
    onUpdateStateChanged: (
      cb: (event: Electron.IpcRendererEvent, state: UpdateStateData) => void
    ) => void
    installUpdate: () => void
    openExternalLink: (link: string) => void
    openFolder: (path: string) => void
    copyToClipboard: (text: string) => void
    openCMPWindow: () => void
    getCoreModule: () => Promise<
      | {
          type: "success"
          port: string
        }
      | {
          type: "error"
          logs: Log[]
        }
      | {
          type: "backwardsMigration"
        }
    >
    getCurrentOS: () => Promise<{
      platform: string
      arch: string
      supportsAutoUpdate: boolean
    }>
    getInitialRuntimePath: () => Promise<string>
    getRuntimePath: () => Promise<string>
    changeRuntimePath: (newPath: string | null) => Promise<void>
    changeRuntimePathProgress: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void
    validateRuntimePath: (
      newPath: string | null
    ) => Promise<"valid" | "invalid" | "potentially_valid">
    skipIntroAnimation: boolean
    closeWindow: () => void
    onShowWindowCloseModal: (cb: () => void) => void
    relaunch: () => void
    listenToCoreModuleProgress: (
      cb: (event: Electron.IpcRendererEvent, progress: number) => void
    ) => void
  }
}

declare module "solid-js" {
  namespace JSX {
    interface IntrinsicElements {
      owadview: {
        class?: string
        adstyle?: string
      }
    }
  }
}

export {}
