import type {
  RuntimeMoveNotice,
  RuntimePathValidation,
  RuntimeRelocationResult
} from "../../main/utils/runtimeRelocation"
import type { ShutdownPromptMessage } from "../../main/utils/shutdownPrompt"
import type { DbRecoveryCategory } from "../../main/utils/dbRecovery"
import type { FEReleaseChannel } from "@gd/core_module/bindings"
import { BoundsSize } from "./utils/adhelper"
import type { Log } from "../../main/coreModule"

import type {
  UpdateStateData,
  CheckForUpdatesResult
} from "../../main/utils/updateState"

declare global {
  interface Window {
    fatalError: (
      error: string | Log[],
      moduleName?: string,
      snapshotPath?: string,
      dbRecovery?: DbRecoveryCategory
    ) => void
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
    openCMPWindow: () => Promise<boolean>
    isCMPWindowAvailable: () => Promise<boolean>
    getCoreModule: () => Promise<
      | {
          type: "success"
          port: string
          apiToken: string
          logs: Log[]
        }
      | {
          type: "error"
          logs: Log[]
          snapshotPath?: string
          dbRecovery?: DbRecoveryCategory
        }
      | {
          type: "backwardsMigration"
          logs: Log[]
        }
    >
    getCurrentOS: () => Promise<{
      platform: string
      arch: string
      supportsAutoUpdate: boolean
    }>
    getInitialRuntimePath: () => Promise<string>
    getRuntimePath: () => Promise<string>
    changeRuntimePath: (
      newPath: string | null,
      switchOnly?: boolean
    ) => Promise<RuntimeRelocationResult>
    // What a move left in the old folder, on every start from the new one until dismissed.
    takeRuntimeMoveNotice: () => Promise<RuntimeMoveNotice | null>
    // The user acknowledged that notice.
    dismissRuntimeMoveNotice: () => Promise<void>
    // The data folder this start could not open, and the default one it uses instead.
    getUnavailableRuntimePath: () => Promise<{
      folder: string
      current: string
    } | null>
    // Keeps that default folder: later starts open it without asking.
    keepDefaultRuntimePath: () => Promise<void>
    changeRuntimePathProgress: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void
    validateRuntimePath: (
      newPath: string | null
    ) => Promise<RuntimePathValidation>
    skipIntroAnimation: boolean
    closeWindow: () => void
    onShowWindowCloseModal: (cb: () => void) => () => void
    /** Resolves "kept" when the user chose to keep GDLauncher open. */
    relaunch: () => Promise<{ status: "kept" | "relaunching" } | undefined>
    onShutdownPrompt: (
      cb: (message: ShutdownPromptMessage) => void
    ) => () => void
    answerShutdownPrompt: (
      id: number,
      choice: "wait" | "force" | "keep"
    ) => void
    /** Prompt `id` is on screen; main asks natively if it is never told. */
    shownShutdownPrompt: (id: number) => void
    setShutdownPromptText: (text: Record<string, string>) => void
    listenToCoreModuleProgress: (
      cb: (event: Electron.IpcRendererEvent, progress: number) => void
    ) => void
    getCurrentProgress: () => Promise<number | null>
    onProtocolUrl: (cb: (url: string) => void) => () => void
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
