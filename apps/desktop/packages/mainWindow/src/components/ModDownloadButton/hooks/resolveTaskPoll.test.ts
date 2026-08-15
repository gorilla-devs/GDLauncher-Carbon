import { describe, it, expect } from "vitest"
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
    downloaded: 0,
    download_total: 0,
    active_subtasks: []
  }) as unknown as FETask

describe("resolveTaskPoll", () => {
  it("still loading (data undefined) is a noop and keeps lastProgress", () => {
    const result = resolveTaskPoll(undefined, knownProgress, true)
    expect(result.action).toEqual({ kind: "noop" })
    expect(result.nextLastProgress).toBe(knownProgress)
  })

  it("Known progress reports rounded percent and records lastProgress", () => {
    const result = resolveTaskPoll(taskWith(knownProgress), null, true)
    expect(result.action).toEqual({ kind: "progress", percent: 42 })
    expect(result.nextLastProgress).toEqual(knownProgress)
  })

  it("Failed progress produces a failed action with the task's error message", () => {
    const result = resolveTaskPoll(taskWith(failedProgress), null, true)
    expect(result.action).toEqual({
      kind: "failed",
      message: "Download failed"
    })
    expect(result.nextLastProgress).toEqual(failedProgress)
  })

  it("Failed progress with no cause falls back to an empty message instead of throwing", () => {
    const result = resolveTaskPoll(taskWith(failedProgressNoCause), null, true)
    expect(result.action).toEqual({ kind: "failed", message: "" })
  })

  it("data === null after a Failed observation does NOT show the success toast", () => {
    const result = resolveTaskPoll(null, failedProgress, true)
    expect(result.action).toEqual({
      kind: "completed",
      showSuccessToast: false
    })
    expect(result.nextLastProgress).toBeNull()
  })

  it("data === null after a non-failed observation shows the success toast for worlds", () => {
    const result = resolveTaskPoll(null, knownProgress, true)
    expect(result.action).toEqual({
      kind: "completed",
      showSuccessToast: true
    })
  })

  it("data === null with no prior progress observed shows the success toast for worlds", () => {
    const result = resolveTaskPoll(null, null, true)
    expect(result.action).toEqual({
      kind: "completed",
      showSuccessToast: true
    })
  })

  it("data === null never shows the success toast for non-world addons", () => {
    const result = resolveTaskPoll(null, knownProgress, false)
    expect(result.action).toEqual({
      kind: "completed",
      showSuccessToast: false
    })
  })
})
