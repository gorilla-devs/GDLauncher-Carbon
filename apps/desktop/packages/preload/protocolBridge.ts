import type { ProtocolDelivery } from "../main/utils/protocolQueue"
let subscriptionRevision = 0
export function subscribeProtocol(
  ipc: {
    on(
      channel: string,
      callback: (event: unknown, value: ProtocolDelivery) => void
    ): unknown
    removeListener(
      channel: string,
      callback: (event: unknown, value: ProtocolDelivery) => void
    ): unknown
    invoke(channel: string): Promise<number | null>
    send(channel: string, value: unknown): void
  },
  callback: (url: string) => void,
  report: (message: string) => void = console.error
): () => void {
  const subscription = String(++subscriptionRevision)
  const accepted = new Set<number>()
  let active = true
  const listener = (_event: unknown, delivery: ProtocolDelivery) => {
    if (
      delivery?.subscription !== subscription ||
      !Number.isSafeInteger(delivery.id)
    )
      return
    if (!accepted.has(delivery.id)) {
      try {
        callback(delivery.url)
        accepted.add(delivery.id)
      } catch {
        report(
          "Protocol action failed; request remains pending. Reopen the window to retry."
        )
        return
      }
    }
    ipc.send("protocol-consumer-ack", delivery)
  }
  ipc.on("protocol-url", listener)
  void ipc
    .invoke("protocol-consumer-generation")
    .then((generation) => {
      if (active && generation !== null)
        ipc.send("protocol-consumer-ready", { subscription, generation })
    })
    .catch(() =>
      report(
        "Protocol consumer could not become ready; requests remain pending"
      )
    )
  return () => {
    active = false
    ipc.removeListener("protocol-url", listener)
    ipc.send("protocol-consumer-revoke", subscription)
  }
}
