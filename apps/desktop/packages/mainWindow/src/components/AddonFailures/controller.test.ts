import { expect, it, vi } from "vitest"
import type { FETask, FETaskOutcome } from "@gd/core_module/bindings"
import {
  acknowledgeCapturedFailure,
  retainedAddonFailures,
  retryCapturedAddonFailure
} from "./controller"

const task = (id: number, name = "InstanceTaskInstallMod", failed = true) =>
  ({
    id,
    name: { translation: name, args: { addon_name: "Broken addon" } },
    progress: failed
      ? {
          type: "Failed",
          value: {
            cause: [{ display: "Checksum mismatch", debug: "details" }],
            backtrace: "trace"
          }
        }
      : { type: "Known", value: 0.4 }
  }) as unknown as FETask

it("discovers retained single/dropdown/row/bulk addon failures after view replacement", () => {
  const snapshot = [
    task(1),
    task(2, "InstanceTaskStageDatapack"),
    task(3, "InstanceTaskInstallWorldCopy"),
    task(4, "ServerTaskInstallMod"),
    task(5, "InstanceTaskLaunch"),
    task(6, "InstanceTaskInstallMod", false)
  ]
  expect(retainedAddonFailures(snapshot).map((task) => task.id)).toEqual([
    1, 2, 3, 4
  ])
  // Independent shell reads rediscover the same manager-owned failures.
  expect(retainedAddonFailures([...snapshot])).toEqual(
    retainedAddonFailures(snapshot)
  )
  expect(snapshot).toHaveLength(6)
})
it("acknowledges only the captured failed ID before shader retry accepts a new task", async () => {
  const calls: string[] = []
  let latestId = 41
  const pending = retryCapturedAddonFailure(
    latestId,
    async (id) => {
      calls.push(`read:${id}`)
      latestId = 42
      return { type: "Failed", value: task(id) }
    },
    async (id) => {
      calls.push(`dismiss:${id}`)
    },
    async () => {
      calls.push(`retry:${latestId}`)
    }
  )
  await pending
  expect(calls).toEqual(["read:41", "dismiss:41", "retry:42"])
})
it.each(["Running", "Succeeded", "Unavailable"] as const)(
  "does not acknowledge %s or run a retry",
  async (type) => {
    const dismiss = vi.fn(),
      retry = vi.fn()
    await expect(
      retryCapturedAddonFailure(
        41,
        async () => ({ type, value: task(41) }) as FETaskOutcome,
        dismiss,
        retry
      )
    ).rejects.toThrow()
    expect(dismiss).not.toHaveBeenCalled()
    expect(retry).not.toHaveBeenCalled()
  }
)
it("read failure leaves the captured failure unacknowledged and blocks retry", async () => {
  const dismiss = vi.fn(),
    retry = vi.fn()
  await expect(
    retryCapturedAddonFailure(
      41,
      async () => {
        throw new Error("read failed")
      },
      dismiss,
      retry
    )
  ).rejects.toThrow("read failed")
  expect(dismiss).not.toHaveBeenCalled()
  expect(retry).not.toHaveBeenCalled()
})
it("an explicitly dismissed failure permits deliberate retry without double dismissal", async () => {
  const dismiss = vi.fn(),
    retry = vi.fn()
  await retryCapturedAddonFailure(
    41,
    async () => ({ type: "Dismissed" }),
    dismiss,
    retry
  )
  expect(dismiss).not.toHaveBeenCalled()
  expect(retry).toHaveBeenCalledOnce()
})
it("dismissal failure prevents acceptance of another shader attempt", async () => {
  const retry = vi.fn()
  await expect(
    retryCapturedAddonFailure(
      41,
      async () => ({ type: "Failed", value: task(41) }),
      async () => {
        throw new Error("dismiss failed")
      },
      retry
    )
  ).rejects.toThrow("dismiss failed")
  expect(retry).not.toHaveBeenCalled()
})
it("explicit panel dismissal targets the selected failure, leaving other failures inspectable", async () => {
  let snapshot = [task(1), task(2)]
  await acknowledgeCapturedFailure(
    1,
    async (id) => ({
      type: "Failed",
      value: snapshot.find((task) => task.id === id)!
    }),
    async (id) => {
      snapshot = snapshot.filter((task) => task.id !== id)
    }
  )
  expect(retainedAddonFailures(snapshot).map((task) => task.id)).toEqual([2])
})
