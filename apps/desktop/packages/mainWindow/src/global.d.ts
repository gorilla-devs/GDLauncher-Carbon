/* eslint-disable no-unused-vars */
/* eslint-disable no-undef */

import type { FEReleaseChannel } from "@gd/core_module/bindings";
import { BoundsSize } from "./utils/adhelper";
import type {
  ProgressInfo,
  UpdateCheckResult,
  UpdateInfo,
} from "electron-updater";

declare global {
  interface Window {
    fatalError: (error: string, moduleName?: string) => void;
    ipcRenderer: import("electron").IpcRenderer;
    report: any;
    getAdSize: () => Promise<BoundsSize>;
    openFileDialog: (filters?: any) => Promise<Electron.OpenDialogReturnValue>;
    openFileDialogExtended: (
      options: Electron.OpenDialogOptions
    ) => Promise<Electron.OpenDialogReturnValue>;
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
    downloadUpdate: () => void;
    openExternalLink: (link: string) => void;
    copyToClipboard: (text: string) => void;
    openCMPWindow: () => void;
    getCoreModulePort: () => Promise<number>;
    getCurrentOS: () => Promise<{ platform: string; arch: string }>;
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
