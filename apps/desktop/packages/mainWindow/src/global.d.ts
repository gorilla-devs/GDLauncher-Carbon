/* eslint-disable no-unused-vars */
/* eslint-disable no-undef */

import type { FEReleaseChannel } from "@gd/core_module/bindings";
import { BoundsSize } from "./utils/adhelper";
import type {
  ProgressInfo,
  UpdateCheckResult,
  UpdateInfo
} from "electron-updater";
import type { Log } from "../../main/coreModule";

declare global {
  interface Window {
    fatalError: (error: string | Log[], moduleName?: string) => void;
    ipcRenderer: import("electron").IpcRenderer;
    report: any;
    getAdSize: () => Promise<BoundsSize>;
    openFileDialog: (
      filters: Electron.OpenDialogOptions
    ) => Promise<Electron.OpenDialogReturnValue>;
    showSaveDialog: (
      filters: Electron.SaveDialogOptions
    ) => Promise<Electron.SaveDialogReturnValue>;
    adSizeChanged: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    checkForUpdates: (
      releaseChannel: FEReleaseChannel
    ) => Promise<UpdateCheckResult | null>;
    onDownloadProgress: (
      cb: (event: Electron.IpcRendererEvent, progressInfo: ProgressInfo) => void
    ) => void;
    updateDownloaded: (cb: (event: Electron.IpcRendererEvent) => void) => void;
    updateAvailable: (
      cb: (event: Electron.IpcRendererEvent, updateInfo: UpdateInfo) => void
    ) => void;
    updateNotAvailable: (
      cb: (event: Electron.IpcRendererEvent) => void
    ) => void;
    installUpdate: () => void;
    openExternalLink: (link: string) => void;
    openFolder: (path: string) => void;
    copyToClipboard: (text: string) => void;
    openCMPWindow: () => void;
    getCoreModule: () => Promise<
      | {
          type: "success";
          port: string;
        }
      | {
          type: "error";
          logs: Log[];
        }
    >;
    getCurrentOS: () => Promise<{ platform: string; arch: string }>;
    getInitialRuntimePath: () => Promise<string>;
    getRuntimePath: () => Promise<string>;
    changeRuntimePath: (newPath: string | null) => Promise<void>;
    changeRuntimePathProgress: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    validateRuntimePath: (
      newPath: string | null
    ) => Promise<"valid" | "invalid" | "potentially_valid">;
    skipIntroAnimation: boolean;
    closeWindow: () => void;
    onShowWindowCloseModal: (cb: () => void) => void;
    relaunch: () => void;
  }
}

declare module "solid-js" {
  namespace JSX {
    interface IntrinsicElements {
      owadview: any;
    }
  }
}

export {};
