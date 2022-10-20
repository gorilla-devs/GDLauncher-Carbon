declare global {
  interface Window {
    clearState: () => void;
    fatalError: (err: Error) => void;
    ipcRenderer: import("electron").IpcRenderer;
    report: any;
  }
}

export {};
