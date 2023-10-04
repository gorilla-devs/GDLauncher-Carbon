import { FEReleaseChannel } from "@gd/core_module/bindings";
import { UpdateInfo } from "electron-updater";
import { createEffect, createSignal } from "solid-js";
import { rspc } from "./rspcClient";

export const [updateAvailable, setUpdateAvailable] =
  createSignal<UpdateInfo | null>(null);

export const [updateProgress, setUpdateProgress] = createSignal(0);
export const [updateDownloaded, setUpdateDownloaded] = createSignal(false);

let lastChannel: FEReleaseChannel | null = null;

window.onDownloadProgress((_, progress) => {
  setUpdateProgress(progress.percent);
});

window.updateDownloaded((_) => {
  setUpdateDownloaded(true);
});

window.updateAvailable((_, result) => {
  console.log("Update available", result);
  setUpdateAvailable(result);
});

window.updateNotAvailable((_) => {
  setUpdateAvailable(null);
});

type IntervalType = ReturnType<typeof setInterval>;

export const checkForUpdates = async () => {
  let interval: null | IntervalType = null;

  let settings = rspc.createQuery(() => ["settings.getSettings"]);

  createEffect(() => {
    if (!settings.data) return;

    if (!lastChannel || settings.data.releaseChannel !== lastChannel) {
      lastChannel = settings.data!.releaseChannel;

      if (interval) {
        clearInterval(interval);
        interval = null;
      }

      window.checkForUpdates(lastChannel!);
      interval = setInterval(
        () => {
          window.checkForUpdates(lastChannel!);
        },
        60 * 30 * 1000
      );
    }
  });
};

export default updateAvailable;
