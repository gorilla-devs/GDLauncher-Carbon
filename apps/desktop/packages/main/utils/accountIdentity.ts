import { hashEmailForOverwolf, type EmailHashes } from "./emailHash"
export interface AccountIdentityEvent {
  session: string
  revision: number
  selected: string | null
  email: string | null
}
/** Local handoff only: the sink performs the existing SDK call when ready. */
export class AccountIdentityReceiver {
  private session = ""
  private revision = -1
  private pending: string | null = null
  private ready = false
  constructor(
    private readonly send: (hashes: EmailHashes | Record<string, never>) => void
  ) {}
  beginCore(session: string) {
    this.session = session
    this.revision = -1
    this.pending = null
    this.apply()
  }
  setReady(ready: boolean) {
    this.ready = ready
    this.apply()
  }
  accept(event: AccountIdentityEvent) {
    if (
      event?.session !== this.session ||
      !Number.isSafeInteger(event.revision) ||
      event.revision <= this.revision ||
      (event.selected !== null && typeof event.selected !== "string") ||
      (event.email !== null && typeof event.email !== "string") ||
      (!event.selected && event.email !== null)
    )
      return
    this.revision = event.revision
    this.pending = event.email
    this.apply()
  }
  acceptPayload(payload: string) {
    try {
      this.accept(JSON.parse(payload) as AccountIdentityEvent)
    } catch {
      /* Ignore malformed core notifications. */
    }
  }
  private apply() {
    if (this.ready)
      this.send(this.pending ? hashEmailForOverwolf(this.pending) : {})
  }
}
export function redactCoreIdentity(text: string): string {
  return text.replace(/(_GDL_ACCOUNT_EMAIL_:)[^\r\n]*/g, "$1<redacted>")
}

/** Frame before logging so a JSON payload split across chunks is still redacted. */
export class CoreOutputLines {
  private tail = ""
  private dropping = false
  push(chunk: string): string[] {
    const lines: string[] = []
    for (const part of chunk.split(/(?<=\n)/)) {
      if (!part) continue
      if (!this.dropping) this.tail += part
      if (this.tail.length > 65536) {
        this.tail = ""
        this.dropping = true
      }
      if (part.endsWith("\n")) {
        if (!this.dropping) lines.push(this.tail.replace(/\r?\n$/, ""))
        this.tail = ""
        this.dropping = false
      }
    }
    return lines
  }
}
