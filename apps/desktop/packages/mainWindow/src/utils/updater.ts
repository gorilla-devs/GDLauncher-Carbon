import { FEReleaseChannel } from "@gd/core_module/bindings";
import { UpdateCheckResult } from "electron-updater";
import { createEffect, createSignal } from "solid-js";
import { rspc } from "./rspcClient";

export const [updateAvailable, setUpdateAvailable] =
  createSignal<UpdateCheckResult | null>(null);

export const [updateProgress, setUpdateProgress] = createSignal(0);
export const [updateDownloaded, setUpdateDownloaded] = createSignal(false);

let lastChannel: FEReleaseChannel | null = null;

window.onDownloadProgress((_, progress) => {
  setUpdateProgress(progress.percent);
});

window.updateDownloaded((_) => {
  setUpdateDownloaded(true);
});

type IntervalType = ReturnType<typeof setInterval>;

export const checkForUpdates = async () => {
  let interval: null | IntervalType = null;

  let settings = rspc.createQuery(() => ["settings.getSettings"]);

  createEffect(() => {
    if (!settings.data) return;

    if (!lastChannel || settings.data.releaseChannel !== lastChannel) {
      lastChannel = settings.data!.releaseChannel;

      const check = async () => {
        const isUpdateAvailable = await window.checkForUpdates(lastChannel!);

        if (isUpdateAvailable) {
          setUpdateAvailable(isUpdateAvailable);
        } else {
          setUpdateAvailable(null);
        }
      };

      if (interval) {
        clearInterval(interval);
        interval = null;
      }

      check();
      interval = setInterval(() => {
        check();
      }, 60 * 30 * 1000);
    }
  });
};

export default updateAvailable;
