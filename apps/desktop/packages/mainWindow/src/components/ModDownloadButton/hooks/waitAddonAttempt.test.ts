import { describe, expect, it } from "vitest"
import type { FETask, FETaskOutcome } from "@gd/core_module/bindings"
import { waitAddonAttempt } from "./waitAddonAttempt"

const running = { progress: { type: "Known", value: 0.8 } } as FETask
const failed = {
  progress: {
    type: "Failed",
    value: { cause: [{ display: "Checksum mismatch" }], backtrace: "" }
  }
} as FETask

describe("captured addon attempt", () => {
  it("waits through acquisition/cache and read failures for the accepted ID", async () => {
    const reads: number[] = []
    const replies: (FETaskOutcome | Error)[] = [
      { type: "Running", value: running },
      new Error("offline"),
      { type: "Running", value: running },
      { type: "Succeeded" }
    ]
    let waits = 0
    const outcome = await waitAddonAttempt(
      73,
      async (id) => {
        reads.push(id)
        const reply = replies.shift()!
        if (reply instanceof Error) throw reply
        return reply
      },
      () => true,
      async () => {
        waits++
      }
    )
    expect(outcome).toEqual({ kind: "completed" })
    expect(reads).toEqual([73, 73, 73, 73])
    expect(waits).toBe(3)
  })
  it("late first observation retains the failure instead of claiming completion", async () => {
    expect(
      await waitAddonAttempt(
        74,
        async () => ({ type: "Failed", value: failed }),
        () => true
      )
    ).toEqual({ kind: "failed", message: "Checksum mismatch" })
  })
  it("does not report a completed old attempt into a changed view", async () => {
    let current = true
    expect(
      await waitAddonAttempt(
        75,
        async () => {
          current = false
          return null
        },
        () => current
      )
    ).toEqual({ kind: "abandoned" })
  })
})

it("does not call a dismissed or missing accepted attempt successful", async () => {
  const result = await waitAddonAttempt(
    99,
    async () => null,
    () => true
  )
  expect(result.kind).not.toBe("completed")
})

it.each(["Dismissed", "Unavailable"] as const)(
  "late %s never certifies success",
  async (type) => {
    expect(
      (
        await waitAddonAttempt(
          99,
          async () => ({ type }),
          () => true
        )
      ).kind
    ).toBe("failed")
  }
)
