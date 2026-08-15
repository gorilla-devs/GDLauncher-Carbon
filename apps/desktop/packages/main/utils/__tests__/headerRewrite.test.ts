import { describe, expect, it, vi } from "vitest"
import {
  handleBeforeSendHeaders,
  handleHeadersReceived,
  upsertKeyValue
} from "../headerRewrite"

describe("upsertKeyValue", () => {
  it("sets the key when absent", () => {
    const obj: Record<string, unknown> = {}
    upsertKeyValue(obj, "Referer", "https://app.gdlauncher.com/")
    expect(obj).toEqual({ Referer: "https://app.gdlauncher.com/" })
  })

  it("keeps an existing value regardless of case", () => {
    const obj: Record<string, unknown> = { referer: "https://existing/" }
    upsertKeyValue(obj, "Referer", "https://app.gdlauncher.com/")
    expect(obj).toEqual({ referer: "https://existing/" })
  })
})

describe("handleBeforeSendHeaders", () => {
  it("invokes callback exactly once when requestHeaders is undefined", () => {
    const callback = vi.fn()
    const details = {
      url: "https://www.youtube.com/embed/abc",
      requestHeaders: undefined
    } as any

    expect(() => handleBeforeSendHeaders(details, callback)).not.toThrow()

    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({
      requestHeaders: { Referer: "https://app.gdlauncher.com/" }
    })
  })

  it("keeps an existing Referer on youtube.com untouched", () => {
    const callback = vi.fn()
    const details = {
      url: "https://youtube.com/watch",
      requestHeaders: { Referer: "https://custom/" }
    } as any

    handleBeforeSendHeaders(details, callback)

    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({
      requestHeaders: { Referer: "https://custom/" }
    })
  })

  it("does not touch requestHeaders for non-YouTube hosts", () => {
    const callback = vi.fn()
    const details = {
      url: "https://example.com/",
      requestHeaders: { "X-Foo": "bar" }
    } as any

    handleBeforeSendHeaders(details, callback)

    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({
      requestHeaders: { "X-Foo": "bar" }
    })
  })

  it("passes through unmodified and does not throw when the url is unparseable", () => {
    const callback = vi.fn()
    const details = {
      url: "not a url",
      requestHeaders: { "X-Foo": "bar" }
    } as any

    expect(() => handleBeforeSendHeaders(details, callback)).not.toThrow()
    expect(callback).toHaveBeenCalledTimes(1)
  })

  it("invokes callback exactly once with unmodified headers on internal error", () => {
    const callback = vi.fn()
    // A getter that throws simulates an unexpected internal failure reading
    // `requestHeaders` itself (as opposed to the inner, already-handled
    // "unparseable URL" case), which the outer try/catch must still turn
    // into a single pass-through callback rather than an unhandled throw.
    const details = {
      url: "https://example.com/",
      get requestHeaders() {
        throw new Error("boom")
      }
    } as any

    expect(() => handleBeforeSendHeaders(details, callback)).not.toThrow()
    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({})
  })
})

describe("handleHeadersReceived", () => {
  it("invokes callback exactly once when responseHeaders is undefined", () => {
    const callback = vi.fn()
    const details = { responseHeaders: undefined } as any

    expect(() => handleHeadersReceived(details, callback)).not.toThrow()

    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({
      responseHeaders: {
        "Access-Control-Allow-Origin": ["*"],
        "Access-Control-Allow-Headers": ["*"]
      }
    })
  })

  it("keeps server-set CORS headers untouched", () => {
    const callback = vi.fn()
    const details = {
      responseHeaders: { "Access-Control-Allow-Origin": ["https://only-me"] }
    } as any

    handleHeadersReceived(details, callback)

    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({
      responseHeaders: {
        "Access-Control-Allow-Origin": ["https://only-me"],
        "Access-Control-Allow-Headers": ["*"]
      }
    })
  })

  it("invokes callback exactly once with unmodified headers on internal error", () => {
    const callback = vi.fn()
    const details = {
      get responseHeaders() {
        throw new Error("boom")
      }
    } as any

    expect(() => handleHeadersReceived(details, callback)).not.toThrow()
    expect(callback).toHaveBeenCalledTimes(1)
    expect(callback).toHaveBeenCalledWith({})
  })
})
