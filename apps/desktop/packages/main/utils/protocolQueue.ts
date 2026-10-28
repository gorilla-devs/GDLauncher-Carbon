export interface ProtocolDelivery {
  id: number
  generation: number
  subscription: string
  url: string
}
export const isSupportedProtocol = (url: string) =>
  ["gdlauncher://", "curseforge://", "modrinth://"].some((prefix) =>
    url.startsWith(prefix)
  )
/** FIFO stays main-owned until the current App callback acknowledges acceptance.
 * A crash after a side effect but before ACK can cause redelivery on next mount. */
export class ProtocolQueue<S> {
  private nextId = 1
  private generation = 0
  private owner: S | null = null
  private consumer: string | null = null
  private queue: { id: number; url: string }[] = []
  private delivered: ProtocolDelivery | null = null
  constructor(
    private readonly send: (sender: S, delivery: ProtocolDelivery) => void
  ) {}
  bind(sender: S) {
    this.owner = sender
    this.revoke(sender)
  }
  revoke(sender: S) {
    if (sender !== this.owner) return
    this.generation++
    this.consumer = null
    this.delivered = null
  }
  close(sender: S) {
    if (sender !== this.owner) return
    this.revoke(sender)
    this.owner = null
  }
  enqueue(url: string) {
    if (!isSupportedProtocol(url)) return
    this.queue.push({ id: this.nextId++, url })
    this.pump()
  }
  session(sender: S) {
    return sender === this.owner ? this.generation : null
  }
  ready(sender: S, subscription: string, generation: number) {
    if (
      sender !== this.owner ||
      generation !== this.generation ||
      typeof subscription !== "string" ||
      !subscription
    )
      return
    if (this.consumer === subscription) return
    this.consumer = subscription
    this.delivered = null
    this.pump()
  }
  unsubscribe(sender: S, subscription: string) {
    if (sender === this.owner && subscription === this.consumer)
      this.revoke(sender)
  }
  ack(sender: S, value: ProtocolDelivery) {
    if (
      sender !== this.owner ||
      // eslint-disable-next-line @typescript-eslint/prefer-optional-chain -- `value` is untrusted IPC input; `?.` on both sides would let a nullish value through
      !this.delivered ||
      value?.id !== this.delivered.id ||
      value.generation !== this.generation ||
      value.subscription !== this.consumer
    )
      return
    this.queue.shift()
    this.delivered = null
    this.pump()
  }
  private pump() {
    if (!this.owner || !this.consumer || this.delivered || !this.queue.length)
      return
    this.delivered = {
      ...this.queue[0],
      generation: this.generation,
      subscription: this.consumer
    }
    try {
      this.send(this.owner, this.delivered)
    } catch {
      this.delivered = null
      console.error("Protocol delivery failed; request remains pending")
    }
  }
}
