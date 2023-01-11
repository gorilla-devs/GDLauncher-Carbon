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
    getMinimumBounds: () => Promise<BoundsSize>;
    minimumBoundsChanged: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    openExternalLink: (link: string) => void;
    copyToClipboard: (text: string) => void;
  }
}

export {};
