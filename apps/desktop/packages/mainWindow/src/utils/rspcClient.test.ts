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
    t: (key: string, options?: { pid?: number | null }) =>
      options?.pid != null ? `${key} ${options.pid}` : key
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

describe("handleGlobalError for a server's record from an earlier session", () => {
  beforeEach(() => {
    toastError.mockClear()
  })
  const refusal = (data: unknown, display = "English refusal") =>
    new Error(
      JSON.stringify({
        cause: [
          { display: "Cannot delete the server" },
          { display, code: "SERVER_RECOVERED_PROCESS", data }
        ],
        backtrace: ""
      })
    )

  it("toasts the translated refusal with its PID instead of the English text", () => {
    handleGlobalError(
      refusal({ kind: "running", pid: 31337, graceful_stop: true }),
      "mutation"
    )
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe(
      "instances:_trn_server_recovered_refusal_running 31337"
    )
  })

  it("names Force stop and the in-game /stop where the server cannot save and stop", () => {
    handleGlobalError(
      refusal({ kind: "running", pid: 31339, graceful_stop: false }),
      "mutation"
    )
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe(
      "instances:_trn_server_recovered_refusal_running_force 31339"
    )
  })

  it("falls back to the core's text for a kind this build does not know", () => {
    handleGlobalError(
      refusal({ kind: "later", pid: 1 }, "Unknown refusal kind 31338"),
      "mutation"
    )
    expect(toastError.mock.calls[0][0]).toBe("Unknown refusal kind 31338")
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
