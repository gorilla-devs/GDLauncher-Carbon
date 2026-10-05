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
    t: (key: string, options?: { pid?: number | null }) =>
      options?.pid != null ? `${key} ${options.pid}` : key
  }
}))

const {
  extractErrorDisplay,
  handleGlobalError,
  parseInvalidateFrame,
  queryClient
} = await import("./rspcClient")
const { LOCAL_ERROR_TOAST, toastMutationError } =
  await import("./mutationErrorToast")

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

// What a refused mutation throws: the rspc body, whose message is the
// pretty-printed core error with its debug text and backtrace.
const corePayload = (cause: object[]) =>
  new Error(
    JSON.stringify({
      code: 500,
      message: JSON.stringify(
        { cause, backtrace: "   0: carbon_app::error::make_rspc" },
        null,
        2
      )
    })
  )

describe("a mutation's own failure toast", () => {
  beforeEach(() => {
    toastError.mockClear()
    toastSuccess.mockClear()
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it("shows the core's sentence under the title, never the rspc JSON", () => {
    toastMutationError(
      "Couldn't resume the setup",
      corePayload([
        { display: "The pack changed after Repair was opened", debug: "X" }
      ])
    )
    expect(toastError).toHaveBeenCalledTimes(1)
    const [title, options] = toastError.mock.calls[0]
    expect(title).toBe("Couldn't resume the setup")
    expect(options.description).toBe("The pack changed after Repair was opened")
    expect(options.description).not.toContain("{")
  })

  it("offers the global toast's Copy action for the sentence it shows", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal("navigator", { clipboard: { writeText } })
    toastMutationError(
      "Couldn't delete the instance",
      corePayload([{ display: "The instance is still running" }])
    )
    const { action } = toastError.mock.calls[0][1]
    expect(action.label).toBe("general:_trn_copy")
    action.onClick()
    await vi.waitFor(() =>
      expect(toastSuccess).toHaveBeenCalledWith(
        "general:_trn_general_copied_to_clipboard"
      )
    )
    expect(writeText).toHaveBeenCalledExactlyOnceWith(
      "The instance is still running"
    )
    // The global toast offers the same action for its own text.
    handleGlobalError(
      corePayload([{ display: "A failure with no local toast" }]),
      "mutation"
    )
    toastError.mock.calls[1][1].action.onClick()
    expect(toastError.mock.calls[1][1].action.label).toBe(action.label)
    expect(writeText).toHaveBeenLastCalledWith("A failure with no local toast")
  })

  it("leaves a server's recovered-process refusal to the global handler", () => {
    const refusal = corePayload([
      { display: "Cannot delete the server" },
      {
        display: "English refusal",
        code: "SERVER_RECOVERED_PROCESS",
        data: { kind: "running", pid: 4242, graceful_stop: false }
      }
    ])
    toastMutationError("Cannot delete server", refusal)
    expect(toastError).not.toHaveBeenCalled()
    // The mutation's meta keeps the global handler's translated refusal.
    queryClient.getMutationCache().config.onError?.(
      refusal,
      undefined,
      undefined,
      {
        meta: LOCAL_ERROR_TOAST
      } as never,
      {} as never
    )
    expect(toastError).toHaveBeenCalledTimes(1)
    // No graceful stop: the page offers Force stop or an in-game /stop.
    expect(toastError.mock.calls[0][0]).toBe(
      "instances:_trn_server_recovered_refusal_running_force 4242"
    )
  })

  it("is the only toast: the global handler skips the plain cause", () => {
    const onError = queryClient.getMutationCache().config.onError!
    onError(
      corePayload([{ display: "Repair is not available right now" }]),
      undefined,
      undefined,
      { meta: LOCAL_ERROR_TOAST } as never,
      {} as never
    )
    onError(
      new Error("not json, a transport failure"),
      undefined,
      undefined,
      {
        meta: LOCAL_ERROR_TOAST
      } as never,
      {} as never
    )
    expect(toastError).not.toHaveBeenCalled()
    // Without the meta, the global toast still shows the cause.
    onError(
      corePayload([{ display: "Repair is not available right now" }]),
      undefined,
      undefined,
      { meta: undefined } as never,
      {} as never
    )
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe(
      "Repair is not available right now"
    )
  })
})

describe("a coded cause with a mutation's own toast", () => {
  const dispatchEvent = vi.fn()
  beforeEach(() => {
    toastError.mockClear()
    vi.stubGlobal("window", { dispatchEvent })
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    dispatchEvent.mockClear()
  })

  // The global handler's toasts and dialogs (dispatched as window events),
  // plus the mutation's own toast, must come to exactly one.
  it.each([
    ["ACCOUNT_BANNED", undefined],
    [
      "SERVER_RECOVERED_PROCESS",
      { kind: "running", pid: 9001, graceful_stop: false }
    ],
    [
      "INSUFFICIENT_MEMORY",
      { instance_id: 1, requested_mb: 8192, needed_mb: 8192, available_mb: 1 }
    ],
    ["INSUFFICIENT_MEMORY", undefined],
    ["EULA_NOT_ACCEPTED", { server_id: 1 }],
    ["EULA_NOT_ACCEPTED", undefined],
    ["A_CODE_THE_HANDLER_DOES_NOT_PRESENT", undefined]
  ])("%s with data %j is shown once", (code, data) => {
    const error = corePayload([
      { display: "Plain cause" },
      { display: "Coded cause", code, data }
    ])
    queryClient
      .getMutationCache()
      .config.onError?.(
        error,
        undefined,
        undefined,
        { meta: LOCAL_ERROR_TOAST } as never,
        {} as never
      )
    toastMutationError("Title", error)
    expect(toastError.mock.calls.length + dispatchEvent.mock.calls.length).toBe(
      1
    )
  })
})

describe("a query that shows its failure in place", () => {
  let consoleError: ReturnType<typeof vi.spyOn>
  beforeEach(() => {
    toastError.mockClear()
    consoleError = vi.spyOn(console, "error").mockImplementation(() => {})
  })
  afterEach(() => {
    queryClient.clear()
    consoleError.mockRestore()
  })

  it("raises no global toast when its meta says it shows the error itself", async () => {
    await queryClient
      .fetchQuery({
        queryKey: ["synthetic-inline-failure"],
        queryFn: () =>
          Promise.reject(new Error("Patch content request failed (404)")),
        meta: { localErrorToast: true }
      })
      .catch(() => {})
    expect(toastError).not.toHaveBeenCalled()
    // Without the meta, the same kind of failure still raises the toast.
    await queryClient
      .fetchQuery({
        queryKey: ["synthetic-global-failure"],
        queryFn: () =>
          Promise.reject(new Error("Patch content request failed (410)"))
      })
      .catch(() => {})
    expect(toastError).toHaveBeenCalledExactlyOnceWith(
      "Patch content request failed (410)",
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
