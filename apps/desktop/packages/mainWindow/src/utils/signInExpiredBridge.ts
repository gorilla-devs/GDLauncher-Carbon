// Event bridge for ACCOUNT_SIGN_IN_EXPIRED error handling
// Since rspcClient runs outside SolidJS tree, we use custom events

export const SIGN_IN_EXPIRED_EVENT = "gdl:sign-in-expired"

export interface SignInExpiredData {
  instance_id: number
}

export function dispatchSignInExpiredEvent(data: SignInExpiredData) {
  window.dispatchEvent(new CustomEvent(SIGN_IN_EXPIRED_EVENT, { detail: data }))
}

export function listenSignInExpired(
  callback: (data: SignInExpiredData) => void
) {
  const handler = (e: Event) => {
    callback((e as CustomEvent<SignInExpiredData>).detail)
  }
  window.addEventListener(SIGN_IN_EXPIRED_EVENT, handler)
  return () => window.removeEventListener(SIGN_IN_EXPIRED_EVENT, handler)
}
