import { describe, expect, it } from "vitest"
import type { FEServerState } from "@gd/core_module/bindings"
import {
  recoveredServerAction,
  recoveredServerPid,
  recoveredServerRefusalKey
} from "./recoveredServer"

describe("a server left behind by an earlier session", () => {
  it("stops at once where the server can save first", () => {
    const state: FEServerState = {
      status: "recoveredRunning",
      process_id: 4242,
      graceful_stop: true
    }
    expect(recoveredServerAction(state)).toBe("stop")
    expect(recoveredServerPid(state)).toBe(4242)
  })

  it("asks before ending a server that cannot save first", () => {
    expect(
      recoveredServerAction({
        status: "recoveredRunning",
        process_id: 4242,
        graceful_stop: false
      })
    ).toBe("forceStop")
  })

  it("offers to clear a record that cannot be verified, with or without a PID", () => {
    for (const process_id of [4242, null]) {
      const state: FEServerState = { status: "recoveredUnverified", process_id }
      expect(recoveredServerAction(state)).toBe("clearRecord")
      expect(recoveredServerPid(state)).toBe(process_id)
    }
  })

  it("leaves every other state to the ordinary Start and Stop", () => {
    const states: FEServerState[] = [
      { status: "stopped", failed_task: null },
      { status: "running", uptime_seconds: 5, process_id: 7 },
      { status: "starting" },
      { status: "stopping" },
      { status: "installing", task_id: 1 },
      { status: "deleting" }
    ]
    for (const state of states) {
      expect(recoveredServerAction(state)).toBeUndefined()
      expect(recoveredServerPid(state)).toBeNull()
    }
    expect(recoveredServerAction(undefined)).toBeUndefined()
  })
})

describe("a refusal because of a record from an earlier session", () => {
  it("has translated text for every kind the core sends", () => {
    expect(
      recoveredServerRefusalKey({
        kind: "running",
        pid: 4242,
        graceful_stop: true
      })
    ).toBe("instances:_trn_server_recovered_refusal_running")
    expect(recoveredServerRefusalKey({ kind: "unverified", pid: 4242 })).toBe(
      "instances:_trn_server_recovered_refusal_unverified"
    )
    expect(recoveredServerRefusalKey({ kind: "unreadable", pid: null })).toBe(
      "instances:_trn_server_recovered_refusal_unreadable"
    )
  })

  it("names Force stop and the in-game /stop for a running server that cannot save and stop", () => {
    // Windows: the server page has no Stop, only Force stop.
    expect(
      recoveredServerRefusalKey({
        kind: "running",
        pid: 4242,
        graceful_stop: false
      })
    ).toBe("instances:_trn_server_recovered_refusal_running_force")
    // Only a running server has a stop to name.
    expect(
      recoveredServerRefusalKey({
        kind: "unverified",
        pid: 4242,
        graceful_stop: false
      })
    ).toBe("instances:_trn_server_recovered_refusal_unverified")
    expect(
      recoveredServerRefusalKey({
        kind: "unreadable",
        pid: null,
        graceful_stop: false
      })
    ).toBe("instances:_trn_server_recovered_refusal_unreadable")
  })

  it("leaves a kind this build does not know to the core's own text", () => {
    for (const data of [
      undefined,
      null,
      {},
      { kind: "later" },
      { kind: "toString" }
    ]) {
      expect(recoveredServerRefusalKey(data)).toBeUndefined()
    }
  })
})
