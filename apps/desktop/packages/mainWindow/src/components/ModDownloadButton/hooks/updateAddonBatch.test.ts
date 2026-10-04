import { expect, it } from "vitest"
import type { FETask } from "@gd/core_module/bindings"
import { ADDON_UPDATE_CONCURRENCY, updateAddonBatch } from "./updateAddonBatch"

it("counts partial failures and waits for the last accepted task to finish", async () => {
  const failed = {
    progress: {
      type: "Failed",
      value: { cause: [{ display: "Required library failed" }] }
    }
  } as FETask
  const running = { progress: { type: "Known", value: 0.99 } } as FETask
  let release!: () => void
  const held = new Promise<void>((resolve) => {
    release = resolve
  })
  const settled: (string | undefined)[] = []
  let lastComplete = false
  let finished = false
  const result = updateAddonBatch(
    [1, 2],
    async (id) => id,
    async (id) =>
      id === 1
        ? { type: "Failed", value: failed }
        : lastComplete
          ? { type: "Succeeded" }
          : { type: "Running", value: running },
    () => true,
    (_, error) => settled.push(error),
    () => held
  ).then((outcome) => {
    finished = true
    return outcome
  })
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(settled).toEqual(["Required library failed"])
  expect(finished).toBe(false)
  lastComplete = true
  release()
  expect(await result).toEqual({ abandoned: false, failed: 1, completed: 2 })
  expect(settled).toEqual(["Required library failed", undefined])
})

it("counts a dismissed or unavailable attempt as failure, including a first observation", async () => {
  const settled: (string | undefined)[] = []
  const result = await updateAddonBatch(
    [1, 2],
    async (id) => id,
    async (id) => ({ type: id === 1 ? "Dismissed" : "Unavailable" }),
    () => true,
    (_, error) => settled.push(error)
  )
  expect(result).toEqual({ abandoned: false, failed: 2, completed: 2 })
  expect(settled.every(Boolean)).toBe(true)
})

it("settles a refused update with the core's sentence, not the rspc payload", async () => {
  const payload = JSON.stringify({
    code: 500,
    message: JSON.stringify({
      cause: [
        { display: "The pack's setup is still pending", debug: "Locked" }
      ],
      backtrace: "   0: carbon_app::managers::instance::mods"
    })
  })
  const settled: (string | undefined)[] = []
  const result = await updateAddonBatch(
    [1],
    async () => {
      throw new Error(payload)
    },
    async () => null,
    () => true,
    (_, error) => settled.push(error)
  )
  expect(result).toEqual({ abandoned: false, failed: 1, completed: 1 })
  expect(settled).toEqual(["The pack's setup is still pending"])
})

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))
const runningTask = {
  type: "Running",
  value: { progress: { type: "Known", value: 0.5 } } as FETask
} as const

it("sends a few updates at once and the next one as soon as one finishes", async () => {
  const accepted: number[] = []
  const settled: number[] = []
  const done = new Set<number>()
  const waits: (() => void)[] = []
  let finished = false
  const result = updateAddonBatch(
    [1, 2, 3, 4, 5, 6],
    async (id) => {
      accepted.push(id)
      return id
    },
    async (id) => (done.has(id) ? { type: "Succeeded" } : runningTask),
    () => true,
    (id) => settled.push(id),
    () => new Promise<void>((resolve) => waits.push(resolve))
  ).then((outcome) => {
    finished = true
    return outcome
  })
  await flush()
  // Not one at a time: the first four run together.
  expect(ADDON_UPDATE_CONCURRENCY).toBe(4)
  expect(accepted).toEqual([1, 2, 3, 4])
  expect(settled).toEqual([])

  done.add(3)
  waits.splice(0).forEach((release) => release())
  await flush()
  expect(settled).toEqual([3])
  // The freed slot takes the next update without waiting for the others.
  expect(accepted).toEqual([1, 2, 3, 4, 5])

  for (const id of [1, 2, 4, 5, 6]) done.add(id)
  for (let round = 0; round < 10 && !finished; round++) {
    waits.splice(0).forEach((release) => release())
    await flush()
  }
  expect(await result).toEqual({ abandoned: false, failed: 0, completed: 6 })
  expect(accepted).toEqual([1, 2, 3, 4, 5, 6])
  expect([...settled].sort()).toEqual([1, 2, 3, 4, 5, 6])
})

it("starts no more updates once the dialog is gone and names the ones it started", async () => {
  const started: number[] = []
  const accepted: number[] = []
  const settled: number[] = []
  const waits: (() => void)[] = []
  let live = true
  const result = updateAddonBatch(
    [1, 2, 3, 4, 5, 6],
    async (id) => {
      accepted.push(id)
      return id
    },
    async () => runningTask,
    () => live,
    (id) => settled.push(id),
    () => new Promise<void>((resolve) => waits.push(resolve)),
    { started: (id) => started.push(id) }
  )
  await flush()
  expect(started).toEqual([1, 2, 3, 4])
  live = false
  waits.splice(0).forEach((release) => release())
  expect(await result).toEqual({ abandoned: true, failed: 0, completed: 0 })
  expect(started).toEqual([1, 2, 3, 4])
  expect(accepted).toEqual([1, 2, 3, 4])
  expect(settled).toEqual([])
})

it("an empty batch finishes at once", async () => {
  const accept = async () => 1
  expect(
    await updateAddonBatch(
      [],
      accept,
      async () => null,
      () => true,
      () => {}
    )
  ).toEqual({ abandoned: false, failed: 0, completed: 0 })
})
