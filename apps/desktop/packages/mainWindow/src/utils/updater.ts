import { UpdateCheckResult } from "electron-updater";
import { createSignal } from "solid-js";

export const [updateAvailable, setUpdateAvailable] =
  createSignal<UpdateCheckResult | null>(null);

export const checkForUpdates = async () => {
  const isUpdateAvailable = await window.checkForUpdates();

  if (isUpdateAvailable) {
    setUpdateAvailable(isUpdateAvailable);
  }
};

export default updateAvailable;
