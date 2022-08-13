export {};

declare global {
  interface Window {
    removeLoading: () => void;
    ipcRenderer: import("electron").IpcRenderer;
  }
}
