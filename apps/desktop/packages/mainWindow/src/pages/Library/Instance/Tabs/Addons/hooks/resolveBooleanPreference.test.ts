import { describe, it, expect, vi } from "vitest"
import { resolveBooleanPreference } from "./resolveBooleanPreference"

describe("resolveBooleanPreference", () => {
  it("resolved-dismissed (data === true, not loading) resolves without awaiting the fallback", async () => {
    const ensureResolved = vi.fn().mockResolvedValue(true)
    const result = await resolveBooleanPreference(
      { isLoading: false, data: true },
      ensureResolved
    )
    expect(result).toBe(true)
    expect(ensureResolved).not.toHaveBeenCalled()
  })

  it("resolved-not-dismissed (data === false, not loading) resolves without awaiting the fallback", async () => {
    const ensureResolved = vi.fn().mockResolvedValue(true)
    const result = await resolveBooleanPreference(
      { isLoading: false, data: false },
      ensureResolved
    )
    expect(result).toBe(false)
    expect(ensureResolved).not.toHaveBeenCalled()
  })

  it("still loading awaits the fallback instead of treating it as false", async () => {
    const ensureResolved = vi.fn().mockResolvedValue(true)
    const result = await resolveBooleanPreference(
      { isLoading: true, data: undefined },
      ensureResolved
    )
    expect(result).toBe(true)
    expect(ensureResolved).toHaveBeenCalledTimes(1)
  })

  it("still loading and the fallback resolves false stays false", async () => {
    const ensureResolved = vi.fn().mockResolvedValue(false)
    const result = await resolveBooleanPreference(
      { isLoading: true, data: undefined },
      ensureResolved
    )
    expect(result).toBe(false)
  })
})
