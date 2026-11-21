import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"

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

const dispatchJavaMismatch = vi.fn()

vi.mock("./javaMismatchBridge", () => ({
  dispatchJavaMismatchEvent: dispatchJavaMismatch
}))

const {
  extractErrorCode,
  extractErrorDisplay,
  handleGlobalError,
  parseInvalidateFrame,
  queryClient,
  rspcErrorCode
} = await import("./rspcClient")

describe("extractErrorCode", () => {
  it("reads the first coded cause", () => {
    const message = JSON.stringify({
      cause: [{ display: "a" }, { display: "b", code: "THEME_INVALID" }],
      backtrace: ""
    })
    expect(extractErrorCode(new Error(message))).toBe("THEME_INVALID")
    expect(extractErrorCode(new Error("nope"))).toBeUndefined()
    expect(extractErrorCode("nope")).toBeUndefined()
  })
})

describe("theme errors", () => {
  afterEach(() => vi.restoreAllMocks())

  it("theme errors are logged once as a warning", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {})
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    toastError.mockClear()
    const message = JSON.stringify({
      cause: [{ display: "bad manifest", code: "THEME_INVALID" }],
      backtrace: "stack"
    })
    handleGlobalError(new Error(message), "query")
    expect(warn).toHaveBeenCalledTimes(1)
    expect(warn.mock.calls[0]).toContain("bad manifest")
    expect(error).not.toHaveBeenCalled()
    expect(toastError).not.toHaveBeenCalled()

    handleGlobalError(new Error("a different plain failure"), "query")
    expect(error).toHaveBeenCalledTimes(1)
  })
})

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

describe("rspcErrorCode", () => {
  it("reads the code of the first cause that has one", () => {
    const message = JSON.stringify({
      cause: [
        { display: "Not enough memory" },
        { display: "Not enough memory", code: "INSUFFICIENT_MEMORY" }
      ],
      backtrace: ""
    })
    expect(rspcErrorCode(new Error(message))).toBe("INSUFFICIENT_MEMORY")
  })

  it("has none for a plain error or a payload without codes", () => {
    expect(rspcErrorCode(new Error("boom"))).toBeUndefined()
    expect(
      rspcErrorCode(new Error(JSON.stringify({ cause: [{ display: "x" }] })))
    ).toBeUndefined()
    expect(rspcErrorCode("not an error")).toBeUndefined()
  })
})

describe("a launch on a Java that doesn't fit", () => {
  it("asks through the crash help dialog instead of a toast", () => {
    toastError.mockClear()
    const data = {
      instanceId: 3,
      instanceName: "Survival",
      neededMajor: 21,
      usedMajor: 17,
      fixMajor: 21
    }
    const message = JSON.stringify({
      cause: [
        {
          display: "Survival is set to Java 17, which doesn't fit it",
          code: "JAVA_MISMATCH",
          data
        }
      ],
      backtrace: ""
    })

    handleGlobalError(new Error(message), "mutation")

    expect(dispatchJavaMismatch).toHaveBeenCalledWith(data)
    expect(toastError).not.toHaveBeenCalled()
  })

  it("is logged at debug level, not as an error", () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {})
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {})
    for (const code of ["JAVA_MISMATCH", "INSUFFICIENT_MEMORY"]) {
      handleGlobalError(
        new Error(
          JSON.stringify({
            cause: [{ display: "x", code }],
            backtrace: ""
          })
        ),
        "mutation"
      )
    }
    expect(errorSpy).not.toHaveBeenCalled()
    expect(debugSpy).toHaveBeenCalledTimes(2)
    handleGlobalError(new Error("boom"), "mutation")
    expect(errorSpy).toHaveBeenCalledTimes(1)
    errorSpy.mockRestore()
    debugSpy.mockRestore()
  })
})

describe("query error toasts", () => {
  beforeEach(() => {
    toastError.mockClear()
  })

  const failQuery = (message: string, meta?: Record<string, unknown>) =>
    queryClient
      .fetchQuery({
        queryKey: ["test.failingQuery", message],
        queryFn: () => Promise.reject(new Error(message)),
        meta
      })
      .catch(() => {})

  it("toasts a failed query by default", async () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {})
    await failQuery("default query failure")
    expect(toastError).toHaveBeenCalledWith(
      "default query failure",
      expect.anything()
    )
    error.mockRestore()
  })

  it("stays quiet for a query that shows its own error (meta.silentError)", async () => {
    await failQuery("silent query failure", { silentError: true })
    expect(toastError).not.toHaveBeenCalled()
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
