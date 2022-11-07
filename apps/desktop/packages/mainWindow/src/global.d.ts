import { BoundsSize } from "./stores/ads";

declare global {
  interface Window {
    clearState: () => void;
    fatalError: (err: Error) => void;
    ipcRenderer: import("electron").IpcRenderer;
    report: any;
    getMinimumBounds: () => Promise<BoundsSize>;
    minimumBoundsChanged: (
      cb: (event: Electron.IpcRendererEvent, ...args: any[]) => void
    ) => void;
  }
}

export {};
