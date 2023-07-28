import { FEReleaseChannel } from "@gd/core_module/bindings";
import { UpdateCheckResult } from "electron-updater";
import { createSignal } from "solid-js";

export const [updateAvailable, setUpdateAvailable] =
  createSignal<UpdateCheckResult | null>(null);

let lastChannel: FEReleaseChannel | null = null;

export const checkForUpdates = async (releaseChannel: FEReleaseChannel) => {
  if (!lastChannel || lastChannel !== releaseChannel) {
    lastChannel = releaseChannel;
    const isUpdateAvailable = await window.checkForUpdates(releaseChannel);

    if (isUpdateAvailable) {
      setUpdateAvailable(isUpdateAvailable);
    } else {
      setUpdateAvailable(null);
    }
  }
};

export default updateAvailable;
