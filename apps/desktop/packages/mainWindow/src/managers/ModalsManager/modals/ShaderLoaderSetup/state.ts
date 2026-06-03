import { createSignal } from "solid-js"

// Bridges the wizard's component-local install phase to the ModalsManager's
// backdrop close handler. `true` while installs are running; both the modal
// layout's X button and the backdrop click check this. We can't gate close
// purely from inside the wizard because the backdrop handler reads the
// registry's `preventClose` value at click time.
export const [shaderInstallRunning, setShaderInstallRunning] =
  createSignal(false)
