import { dbRecoveryCategory, type DbRecoveryCategory } from "./dbRecovery"
import { StringDecoder } from "node:string_decoder"
import { redactCoreIdentity } from "./accountIdentity"

export interface CoreLog {
  type: "info" | "error"
  message: string
}
export type CoreStartup =
  | {
      type: "success"
      result: { port: number; apiToken: string }
      logs: CoreLog[]
    }
  | {
      type: "error"
      logs: CoreLog[]
      snapshotPath?: string
      dbRecovery?: DbRecoveryCategory
    }
  | { type: "backwardsMigration"; logs: CoreLog[] }
export const MAX_CORE_RECORD = 65536
export const MAX_STARTUP_CAPTURE = 1024 * 1024
const OVERFLOW = "[CORE] Overlong stdout record discarded"

/** One decoder/framer owns raw dispatch and every sanitized stdout sink. CR,
 * LF and CRLF terminate records; unterminated EOF never grants protocol authority. */
export function createCoreOutput(sinks: {
  log: (log: CoreLog) => void
  settle: (result: CoreStartup) => void
  record: (raw: string) => void
  progress: (event: string) => void
}) {
  const decoder = new StringDecoder("utf8")
  let tail = "",
    dropping = false,
    afterCR = false,
    settled = false
  let captureSize = 0,
    captureFull = false
  const logs: CoreLog[] = []
  function capture(log: CoreLog) {
    if (settled || captureFull) return
    if (captureSize + log.message.length > MAX_STARTUP_CAPTURE) {
      captureFull = true
      logs.push({
        type: "error",
        message: "[CORE] Startup capture limit reached"
      })
    } else {
      logs.push(log)
      captureSize += log.message.length
    }
  }
  function finish(result: Omit<CoreStartup, "logs">) {
    if (settled) return
    settled = true
    sinks.settle({ ...result, logs: logs.slice() } as CoreStartup)
  }
  function output(raw: string, complete: boolean) {
    const message = redactCoreIdentity(raw).replace(
      /(_STATUS_:READY\|)[^\r\n]*/g,
      "$1<redacted>"
    )
    const log: CoreLog = { type: "info", message }
    sinks.log(log)
    capture(log)
    if (!complete) return
    if (raw.startsWith("_STATUS_:")) {
      const [event, payload, apiToken] = raw
        .slice("_STATUS_:".length)
        .split("|")
      if (event === "READY") {
        const port = Number(payload)
        if (!apiToken || !Number.isInteger(port) || port < 1 || port > 65535) {
          capture({
            type: "error",
            message: "Core module did not provide valid readiness credentials"
          })
          finish({ type: "error" })
        } else
          finish({ type: "success", result: { port, apiToken } } as CoreStartup)
      } else if (event === "BACKWARDS_MIGRATION")
        finish({ type: "backwardsMigration" })
      else if (event === "DB_DOWNGRADED") {
        /* Nonfatal: recorded startup diagnostic; startup continues. */
      } else if (
        [
          "DB_MIGRATION_FAILED",
          "DB_DIVERGED",
          "DB_CORRUPT",
          "DB_DOWNGRADE_FAILED"
        ].includes(event)
      ) {
        capture({ type: "error", message: `Database error: ${event}` })
        finish({
          type: "error",
          dbRecovery: dbRecoveryCategory(event, payload),
          snapshotPath: event === "DB_DOWNGRADE_FAILED" ? payload : undefined
        } as CoreStartup)
      } else sinks.progress(event)
    } else sinks.record(raw)
  }
  function frame(text: string) {
    for (const ch of text) {
      if (afterCR) {
        afterCR = false
        if (ch === "\n") continue
      }
      if (ch === "\r" || ch === "\n") {
        if (dropping) {
          const log: CoreLog = { type: "error", message: OVERFLOW }
          sinks.log(log)
          capture(log)
        } else output(tail, true)
        tail = ""
        dropping = false
        afterCR = ch === "\r"
      } else if (!dropping) {
        tail += ch
        if (tail.length > MAX_CORE_RECORD) {
          tail = ""
          dropping = true
        }
      }
    }
  }
  return {
    push: (bytes: Buffer) => frame(decoder.write(bytes)),
    end() {
      frame(decoder.end())
      if (dropping) {
        const log: CoreLog = { type: "error", message: OVERFLOW }
        sinks.log(log)
        capture(log)
      } else if (tail) output(tail, false)
      tail = ""
      dropping = false
    },
    stderr(message: string) {
      capture({ type: "error", message })
    },
    fail(message: string) {
      capture({ type: "error", message })
      finish({ type: "error" })
    },
    get settled() {
      return settled
    }
  }
}
