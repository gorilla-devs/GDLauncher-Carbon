import { createSignal } from "solid-js"

// Shared with the settings page that opens the modal and with the
// ModalsManager registry's preventClose hook. While true, the backdrop
// click and the modal's close button are both blocked — closing
// mid-migration would orphan files between the old and new runtime path.
export const [isChangingRuntimePath, setIsChangingRuntimePath] =
  createSignal(false)
