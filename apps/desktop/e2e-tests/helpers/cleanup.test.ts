import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { reportCleanupFailure, withCleanup } from "./cleanup.js"

describe("withCleanup", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {})
  })
  afterEach(() => {
    consoleErrorSpy.mockRestore()
  })

  // The four quadrants of (body ok/fail) x (cleanup ok/fail) — the whole
  // point of this helper is picking the right outcome in each.
  it("body ok, cleanup ok: resolves with the body's value, no console.error", async () => {
    const cleanup = vi.fn().mockResolvedValue(undefined)

    await expect(
      withCleanup(async () => "result", cleanup, "cleanup failed")
    ).resolves.toBe("result")

    expect(cleanup).toHaveBeenCalledTimes(1)
    expect(consoleErrorSpy).not.toHaveBeenCalled()
  })

  it("body ok, cleanup fails: rethrows the cleanup error, not masked", async () => {
    const cleanupError = new Error("cleanup boom")
    const cleanup = vi.fn().mockRejectedValue(cleanupError)

    await expect(
      withCleanup(async () => "result", cleanup, "cleanup failed")
    ).rejects.toBe(cleanupError)

    expect(consoleErrorSpy).not.toHaveBeenCalled()
  })

  it("body fails, cleanup ok: rethrows the body's original error", async () => {
    const bodyError = new Error("body boom")
    const cleanup = vi.fn().mockResolvedValue(undefined)

    await expect(
      withCleanup(
        async () => {
          throw bodyError
        },
        cleanup,
        "cleanup failed"
      )
    ).rejects.toBe(bodyError)

    expect(cleanup).toHaveBeenCalledTimes(1)
    expect(consoleErrorSpy).not.toHaveBeenCalled()
  })

  it("body fails, cleanup also fails: rethrows the body's error and logs the cleanup error", async () => {
    const bodyError = new Error("body boom")
    const cleanupError = new Error("cleanup boom")
    const cleanup = vi.fn().mockRejectedValue(cleanupError)

    await expect(
      withCleanup(
        async () => {
          throw bodyError
        },
        cleanup,
        "cleanup for X also failed:"
      )
    ).rejects.toBe(bodyError)

    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "cleanup for X also failed:",
      cleanupError
    )
  })

  // A literal `throw undefined` from the body must still read as "the body
  // failed", not be mistaken for a passing body via an "error is undefined"
  // sentinel — see the doc comment on withCleanup.
  it("treats a body that throws undefined as a failure, not a pass", async () => {
    const cleanupError = new Error("cleanup boom")
    const cleanup = vi.fn().mockRejectedValue(cleanupError)

    await expect(
      withCleanup(
        async () => {
          // eslint-disable-next-line @typescript-eslint/only-throw-error
          throw undefined
        },
        cleanup,
        "cleanup for Y also failed:"
      )
    ).rejects.toBeUndefined()

    // A passing body would have rethrown cleanupError instead of logging it.
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "cleanup for Y also failed:",
      cleanupError
    )
  })

  it("the bodyFailed-callback form passes the correct flag through in all four quadrants", async () => {
    const seen: boolean[] = []

    await withCleanup(
      async () => "ok",
      async (alreadyFailed) => {
        seen.push(alreadyFailed)
      }
    )
    expect(seen).toEqual([false])

    await expect(
      withCleanup(
        async () => {
          throw new Error("boom")
        },
        async (alreadyFailed) => {
          seen.push(alreadyFailed)
        }
      )
    ).rejects.toThrow("boom")
    expect(seen).toEqual([false, true])
  })
})

describe("reportCleanupFailure", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {})
  })
  afterEach(() => {
    consoleErrorSpy.mockRestore()
  })

  it("rethrows when the body has not already failed", () => {
    const cleanupError = new Error("cleanup boom")
    expect(() => reportCleanupFailure(cleanupError, false, "message")).toThrow(
      cleanupError
    )
    expect(consoleErrorSpy).not.toHaveBeenCalled()
  })

  it("logs instead of throwing when the body already failed", () => {
    const cleanupError = new Error("cleanup boom")
    expect(() =>
      reportCleanupFailure(cleanupError, true, "message")
    ).not.toThrow()
    expect(consoleErrorSpy).toHaveBeenCalledWith("message", cleanupError)
  })
})
