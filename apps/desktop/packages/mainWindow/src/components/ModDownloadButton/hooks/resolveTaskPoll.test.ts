import { describe, it, expect, vi } from "vitest"

// Without a loaded bundle the key stands in, with the file name it was given.
vi.mock("@gd/i18n", () => ({
  i18n: {
    t: (key: string, options?: { filename?: string }) =>
      options?.filename ? `${key} ${options.filename}` : key
  }
}))
import { resolveTaskPoll } from "./resolveTaskPoll"
import type { FETask, Progress } from "@gd/core_module/bindings"

const knownProgress: Progress = { type: "Known", value: 0.42 }
const failedProgress: Progress = {
  type: "Failed",
  value: {
    cause: [{ display: "Download failed", debug: "debug" }],
    backtrace: ""
  }
}
const failedProgressNoCause: Progress = {
  type: "Failed",
  value: { cause: [], backtrace: "" }
}

const taskWith = (progress: Progress): FETask =>
  ({
    id: 1,
    name: { type: "Literal", value: "test" } as never,
    progress,
    downloaded: "0",
    download_total: "0",
    active_subtasks: []
  }) as unknown as FETask

describe("resolveTaskPoll", () => {
  it("still loading (data undefined) is a noop and keeps lastProgress", () => {
    const result = resolveTaskPoll(undefined, knownProgress, true)
    expect(result.action).toEqual({ kind: "noop" })
    expect(result.nextLastProgress).toBe(knownProgress)
  })

  it("Known progress reports rounded percent and records lastProgress", () => {
    const result = resolveTaskPoll(
      { type: "Running", value: taskWith(knownProgress) },
      null,
      true
    )
    expect(result.action).toEqual({ kind: "progress", percent: 42 })
    expect(result.nextLastProgress).toEqual(knownProgress)
  })

  it("Failed progress produces a failed action with the task's error message", () => {
    const result = resolveTaskPoll(
      { type: "Failed", value: taskWith(failedProgress) },
      null,
      true
    )
    expect(result.action).toEqual({
      kind: "failed",
      message: "Download failed"
    })
    expect(result.nextLastProgress).toEqual(failedProgress)
  })

  it.each([
    [
      "the sentence of its refusal's code",
      { display: "Selected world no longer exists", code: "WORLD_GONE" },
      "content:_trn_worlds.refusal.world_gone"
    ],
    [
      "the file its refusal names",
      {
        display: "Failed to download data pack a.zip: timed out",
        code: "DATAPACK_DOWNLOAD_FAILED",
        data: { filename: "Tom & Jerry.zip", cause: "timed out" }
      },
      "content:_trn_worlds.refusal.download_failed Tom & Jerry.zip"
    ],
    [
      "the core's sentence for a code it does not know",
      { display: "Something newer", code: "A_CODE_FROM_A_NEWER_CORE" },
      "Something newer"
    ]
  ])("a failed task is worded with %s", (_, cause, message) => {
    const progress: Progress = {
      type: "Failed",
      value: { cause: [{ debug: "debug", ...cause }], backtrace: "" }
    }
    expect(
      resolveTaskPoll({ type: "Failed", value: taskWith(progress) }, null, true)
        .action
    ).toEqual({ kind: "failed", message, code: cause.code })
  })

  it("Failed progress with no cause falls back to an empty message instead of throwing", () => {
    const result = resolveTaskPoll(
      { type: "Failed", value: taskWith(failedProgressNoCause) },
      null,
      true
    )
    expect(result.action).toEqual({ kind: "failed", message: "" })
  })

  it.each([null, { type: "Unavailable" } as const])(
    "missing history never certifies success: %j",
    (outcome) => {
      expect(resolveTaskPoll(outcome, knownProgress, true).action.kind).toBe(
        "failed"
      )
    }
  )
  it("dismissed failure is a distinct stop action even before the first poll", () => {
    expect(resolveTaskPoll({ type: "Dismissed" }, null, true).action).toEqual({
      kind: "dismissed"
    })
  })
  it("only explicit success shows the requested completion toast", () => {
    expect(resolveTaskPoll({ type: "Succeeded" }, null, true).action).toEqual({
      kind: "completed",
      showSuccessToast: true
    })
    expect(
      resolveTaskPoll({ type: "Succeeded" }, knownProgress, false).action
    ).toEqual({ kind: "completed", showSuccessToast: false })
  })
})
