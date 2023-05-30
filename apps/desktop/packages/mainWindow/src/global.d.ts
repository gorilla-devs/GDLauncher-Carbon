/* eslint-disable no-unused-vars */
/* eslint-disable no-undef */

import { BoundsSize } from "./utils/adhelper";

declare global {
  interface Window {
    fatalError: (error: string, moduleName?: string) => void;
    ipcRenderer: import("electron").IpcRenderer;
    report: any;
    getAdSize: () => Promise<BoundsSize>;
    openFileDialog: (filters?: any) => Promise<Electron.OpenDialogReturnValue>;
    adSizeChanged: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    checkUpdate: () => void;
    installUpdate: () => void;
    updateAvailable: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
    releaseChannel: (releaseChannel: string) => void;
    openExternalLink: (link: string) => void;
    copyToClipboard: (text: string) => void;
    getCoreModulePort: () => Promise<number>;
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
