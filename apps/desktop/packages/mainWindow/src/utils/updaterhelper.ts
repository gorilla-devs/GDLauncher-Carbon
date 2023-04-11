import { createSignal } from "solid-js";

export const [updateAvailable, setUpdateAvailable] = createSignal(false);

const init = async () => {
  window.updateAvailable(() => {
    setUpdateAvailable(true);
  });
};

init();

export default updateAvailable;
