import { createSignal } from "solid-js";

export const [updateAvailable, setUpdateAvailable] = createSignal(false);

export const checkForUpdates = () => {
  window.checkUpdate();
  window.updateAvailable(() => {
    setUpdateAvailable(true);
  });
};

export default updateAvailable;
