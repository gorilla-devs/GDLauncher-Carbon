import { describe, expect, it, vi } from "vitest"
import { runShaderSteps } from "./pipeline"

describe("shader Auto completion", () => {
  it("does not complete when a disappeared task left the required loader disabled", async () => {
    const complete = vi.fn()
    await expect(
      runShaderSteps(
        [async () => 7],
        async () => {},
        async () => false,
        false,
        () => false,
        complete
      )
    ).rejects.toThrow()
    expect(complete).not.toHaveBeenCalled()
  })
  it("completes after enabled artifact verification", async () => {
    const complete = vi.fn()
    await runShaderSteps(
      [async () => 7, async () => 8],
      async () => {},
      async () => true,
      false,
      () => false,
      complete
    )
    expect(complete).toHaveBeenCalledWith(8)
  })
  it("Continue anyway installs only the file without requiring or enabling a loader", async () => {
    const verify = vi.fn()
    const complete = vi.fn()
    await runShaderSteps(
      [async () => 8],
      async () => {},
      verify,
      true,
      () => false,
      complete
    )
    expect(verify).not.toHaveBeenCalled()
    expect(complete).toHaveBeenCalledWith(8)
  })
  it("stops before shader installation when exact loader verification fails", async () => {
    const shader = vi.fn().mockResolvedValue(9)
    const complete = vi.fn()
    await expect(
      runShaderSteps(
        [async () => 7, shader],
        async () => {},
        async () => true,
        false,
        () => false,
        complete,
        async (index) => {
          if (index === 0) throw new Error("loader remains disabled")
        }
      )
    ).rejects.toThrow("loader remains disabled")
    expect(shader).not.toHaveBeenCalled()
    expect(complete).not.toHaveBeenCalled()
  })
  it("rechecks after shader installation before completing Auto", async () => {
    const complete = vi.fn()
    const afterTask = vi.fn().mockResolvedValue(undefined)
    await expect(
      runShaderSteps(
        [async () => 7, async () => 9],
        async () => {},
        async () => false,
        false,
        () => false,
        complete,
        afterTask
      )
    ).rejects.toThrow()
    expect(afterTask).toHaveBeenCalledTimes(2)
    expect(complete).not.toHaveBeenCalled()
  })
})
