declare global {
  interface Window {
    removeLoading: () => void;
    fatalError: (err: Error) => void;
    ipcRenderer: import("electron").IpcRenderer;
    plausible: any;
  }
}

export {};
