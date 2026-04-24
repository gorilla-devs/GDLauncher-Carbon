// Event bridge for EULA_NOT_ACCEPTED error handling
// Since rspcClient runs outside SolidJS tree, we use custom events

export const SERVER_EULA_EVENT = "gdl:server-eula-required"

export interface ServerEulaData {
  server_id: number
}

export function dispatchServerEulaEvent(data: ServerEulaData) {
  window.dispatchEvent(new CustomEvent(SERVER_EULA_EVENT, { detail: data }))
}

export function listenServerEula(callback: (data: ServerEulaData) => void) {
  const handler = (e: Event) => {
    callback((e as CustomEvent<ServerEulaData>).detail)
  }
  window.addEventListener(SERVER_EULA_EVENT, handler)
  return () => window.removeEventListener(SERVER_EULA_EVENT, handler)
}
