import { createSignal } from "solid-js"

// Bridges the modal's component-local phase to the ModalsManager's backdrop
// close handler. `true` while a cleanup is actively running; both the modal's
// own X button and the backdrop click check this to decide whether to close.
export const [cleanupRunning, setCleanupRunning] = createSignal(false)
