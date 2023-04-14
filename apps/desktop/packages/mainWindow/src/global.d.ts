/* eslint-disable no-unused-vars */
/* eslint-disable no-undef */

import { BoundsSize } from "./utils/adhelper";

declare global {
  interface Window {
    clearLoading: () => void;
    fatalError: (error: string, moduleName?: string) => void;
    updateLoading: (loaded: number, total: number) => void;
    ipcRenderer: import("electron").IpcRenderer;
    report: any;
    getAdSize: () => Promise<BoundsSize>;
    adSizeChanged: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    checkUpdate: () => void;
    installUpdate: () => void;
    updateAvailable: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    openExternalLink: (link: string) => void;
    copyToClipboard: (text: string) => void;
    getCoreModuleStatus: () => Promise<void>;
  }
}

export {};
