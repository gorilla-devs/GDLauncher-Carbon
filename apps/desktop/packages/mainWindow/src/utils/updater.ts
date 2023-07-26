import { FEReleaseChannel } from "@gd/core_module/bindings";
import { UpdateCheckResult } from "electron-updater";
import { createSignal } from "solid-js";

export const [updateAvailable, setUpdateAvailable] =
  createSignal<UpdateCheckResult | null>(null);

let init = false;

export const checkForUpdates = async (releaseChannel: FEReleaseChannel) => {
  if (!init) {
    init = true;
    const isUpdateAvailable = await window.checkForUpdates(releaseChannel);

    if (isUpdateAvailable) {
      setUpdateAvailable(isUpdateAvailable);
    }
  }
};

export default updateAvailable;
