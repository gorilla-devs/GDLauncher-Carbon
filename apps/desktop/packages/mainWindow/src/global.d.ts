declare global {
  interface Window {
    clearState: () => void;
    fatalError: (err: Error) => void;
    ipcRenderer: import("electron").IpcRenderer;
    plausible: any;
  }
}

export {};
