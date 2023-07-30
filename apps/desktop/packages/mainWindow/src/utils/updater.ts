import { FEReleaseChannel } from "@gd/core_module/bindings";
import { UpdateCheckResult } from "electron-updater";
import { createSignal } from "solid-js";

export const [updateAvailable, setUpdateAvailable] =
  createSignal<UpdateCheckResult | null>(null);

export const [updateProgress, setUpdateProgress] = createSignal(0);

let lastChannel: FEReleaseChannel | null = null;

window.onDownloadProgress((_, progress) => {
  setUpdateProgress(progress.percent);
});

export const checkForUpdates = async (releaseChannel: FEReleaseChannel) => {
  let interval = null;

  const check = async () => {
    lastChannel = releaseChannel;
    const isUpdateAvailable = await window.checkForUpdates(releaseChannel);

    if (isUpdateAvailable) {
      setUpdateAvailable(isUpdateAvailable);
    } else {
      setUpdateAvailable(null);
    }
  };

  if (!lastChannel || lastChannel !== releaseChannel) {
    if (interval) {
      clearInterval(interval);
    }
    check();
    interval = setInterval(() => {
      check();
    }, 60 * 30 * 1000);
  }
};

export default updateAvailable;
