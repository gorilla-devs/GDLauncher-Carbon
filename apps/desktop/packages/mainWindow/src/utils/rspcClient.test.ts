import { describe, it, expect, vi, beforeEach } from "vitest"

const toastError = vi.fn()
const toastSuccess = vi.fn()

vi.mock("@gd/ui", () => ({
  toast: {
    error: toastError,
    success: toastSuccess
  }
}))

vi.mock("@gd/i18n", () => ({
  i18n: {
    t: (key: string) => key
  }
}))

const { extractErrorDisplay, handleGlobalError, parseInvalidateFrame } =
  await import("./rspcClient")

describe("extractErrorDisplay", () => {
  it("falls back to the raw text for a bare JSON literal instead of throwing", () => {
    expect(() => extractErrorDisplay(new Error("null"))).not.toThrow()
    expect(extractErrorDisplay(new Error("null"))).toBe("null")
  })

  it("still prefers the cause display for a well-formed rspc payload", () => {
    const message = JSON.stringify({
      cause: [{ display: "Something broke" }],
      backtrace: ""
    })
    expect(extractErrorDisplay(new Error(message))).toBe("Something broke")
  })
})

describe("handleGlobalError", () => {
  beforeEach(() => {
    toastError.mockClear()
    toastSuccess.mockClear()
  })

  it("message 'null' surfaces a fallback toast instead of throwing", () => {
    expect(() => handleGlobalError(new Error("null"), "query")).not.toThrow()
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe("null")
  })

  it("a genuinely unparsable message still toasts the raw text", () => {
    handleGlobalError(new Error("not json at all"), "mutation")
    expect(toastError).toHaveBeenCalledWith(
      "not json at all",
      expect.anything()
    )
  })

  it("a well-formed cause payload toasts the cause display", () => {
    const message = JSON.stringify({
      cause: [{ display: "Download failed" }],
      backtrace: ""
    })
    handleGlobalError(new Error(message), "query")
    expect(toastError).toHaveBeenCalledWith(
      "Download failed",
      expect.anything()
    )
  })
})

describe("parseInvalidateFrame", () => {
  it("returns undefined instead of throwing on a malformed frame", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    expect(() => parseInvalidateFrame("{not json")).not.toThrow()
    expect(parseInvalidateFrame("{not json")).toBeUndefined()
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })

  it("parses a well-formed frame", () => {
    const frame = JSON.stringify({ key: "instance.getAll", args: null })
    expect(parseInvalidateFrame(frame)).toEqual({
      key: "instance.getAll",
      args: null
    })
  })
})
