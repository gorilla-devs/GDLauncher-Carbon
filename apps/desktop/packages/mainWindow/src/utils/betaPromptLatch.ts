import { createSignal } from "solid-js"

// app.tsx opens the beta prompt at most once per session. shouldShowBetaPrompt
// never turns false within a session, so anything that waits for the prompt
// (the seasonal dialog) must wait only until it has been opened, then rely on
// the modal stack.
const [betaPromptChecked, setBetaPromptChecked] = createSignal(false)

export { betaPromptChecked, setBetaPromptChecked }
