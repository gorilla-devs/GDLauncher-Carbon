// Event bridge for ACCOUNT_BANNED error handling
// Since rspcClient runs outside SolidJS tree, we use custom events

export const ACCOUNT_BANNED_EVENT = "gdl:account-banned"

// Dispatch from rspcClient when ACCOUNT_BANNED detected
export function dispatchBannedEvent() {
  window.dispatchEvent(new CustomEvent(ACCOUNT_BANNED_EVENT))
}
