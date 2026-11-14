import { expect, it } from "vitest"
import { EventEmitter } from "node:events"
import { coreExit, subscribeCoreExit } from "./coreExitBridge"

it("passes on only an exit main reported", () => {
  expect(coreExit({ code: 101, signal: null })).toEqual({
    code: 101,
    signal: null
  })
  expect(coreExit({ code: null, signal: "SIGKILL", extra: 1 })).toEqual({
    code: null,
    signal: "SIGKILL"
  })
  expect(coreExit({ code: "1", signal: 9 })).toEqual({
    code: null,
    signal: null
  })
  for (const none of [null, undefined, false, "exit", 0])
    expect(coreExit(none)).toBeNull()
})

it("tells the page of an exit without the IPC event, until unsubscribed", () => {
  const ipc = new EventEmitter()
  const seen: unknown[][] = []
  const stop = subscribeCoreExit(ipc, (...args) => seen.push(args))
  ipc.emit("coreExited", { sender: {} }, { code: 101, signal: null })
  ipc.emit("coreExited", { sender: {} }, null)
  expect(seen).toEqual([[{ code: 101, signal: null }]])
  stop()
  ipc.emit("coreExited", { sender: {} }, { code: 1, signal: null })
  expect(seen).toHaveLength(1)
  expect(ipc.listenerCount("coreExited")).toBe(0)
})
