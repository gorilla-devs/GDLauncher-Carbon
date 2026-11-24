import {
  describe,
  it,
  expect,
  vi,
  beforeEach,
  afterEach,
  type MockInstance
} from "vitest"

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
    t: (
      key: string,
      options?: { pid?: number | null; version?: string; filename?: string }
    ) =>
      options?.pid != null
        ? `${key} ${options.pid}`
        : options?.version
          ? `${key} ${options.version}`
          : options?.filename
            ? `${key} ${options.filename}`
            : key
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
const { LOCAL_ERROR_TOAST, toastMutationError } =
  await import("./mutationErrorToast")

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

describe("handleGlobalError once the background service stopped", () => {
  beforeEach(() => {
    toastError.mockClear()
  })
  afterEach(async () => {
    vi.unstubAllGlobals()
    ;(await import("./coreStopped")).resetCoreStopped()
  })

  it("no longer reports failed requests: they are not a network problem", async () => {
    const { watchCoreStopped } = await import("./coreStopped")
    let report!: (exit: { code: number | null; signal: string | null }) => void
    vi.stubGlobal("window", {
      onCoreExited: (cb: typeof report) => {
        report = cb
        return () => {}
      },
      coreExit: async () => null
    })
    watchCoreStopped()
    handleGlobalError(new TypeError("Failed to fetch"), "query")
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe("errors:_trn_network_error_toast")

    toastError.mockClear()
    report({ code: 101, signal: null })
    handleGlobalError(new TypeError("Failed to fetch"), "mutation")
    handleGlobalError(new Error("anything else"), "mutation")
    expect(toastError).not.toHaveBeenCalled()
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

describe("a data pack install into a world refused for the game version", () => {
  beforeEach(() => {
    toastError.mockClear()
  })
  const refusal = (data: unknown) =>
    corePayload([
      {
        display: "This project has no data pack for Minecraft 1.20.1",
        code: "NO_DATAPACK_FOR_VERSION",
        data
      }
    ])

  it("toasts the translated refusal with the version", () => {
    // The world install mutation has no toast of its own.
    queryClient
      .getMutationCache()
      .config.onError?.(
        refusal({ version: "1.20.1" }),
        undefined,
        undefined,
        { meta: undefined } as never,
        {} as never
      )
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe(
      "content:_trn_worlds.no_datapack_for_version 1.20.1"
    )
  })

  it("falls back to the core's text without a version", () => {
    handleGlobalError(refusal(undefined), "mutation")
    expect(toastError.mock.calls[0][0]).toBe(
      "This project has no data pack for Minecraft 1.20.1"
    )
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
    ["ACCOUNT_SIGN_IN_EXPIRED", { instance_id: 1 }],
    ["ACCOUNT_SIGN_IN_EXPIRED", undefined],
    ["NO_DATAPACK_FOR_VERSION", { version: "1.19.4" }],
    ["NO_DATAPACK_FOR_VERSION", undefined],
    ["INSTANCE_GAME_ACTIVE", undefined],
    ["DATAPACK_GONE", undefined],
    ["DATAPACK_DOWNLOAD_FAILED", { filename: "pack.zip", cause: "timed out" }],
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

describe("a refused change to a world or its data packs", () => {
  // Spied on per test: another group here restores every mock after its own.
  let invalidate: MockInstance<typeof queryClient.invalidateQueries>
  beforeEach(() => {
    toastError.mockClear()
    invalidate = vi
      .spyOn(queryClient, "invalidateQueries")
      .mockResolvedValue(undefined)
  })
  afterEach(() => {
    invalidate.mockRestore()
  })
  const refusal = (code: string, display: string, data?: unknown) =>
    corePayload([
      { display: "Could not change the world" },
      { display, code, data }
    ])
  // These mutations have no toast of their own.
  const refuse = (error: Error, variables?: unknown) =>
    queryClient
      .getMutationCache()
      .config.onError?.(
        error,
        variables,
        undefined,
        { meta: undefined } as never,
        {} as never
      )

  it("is told once, in the player's language", () => {
    refuse(refusal("WORLD_GONE", "Selected world no longer exists"), {
      instance_id: 5
    })
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toBe(
      "content:_trn_worlds.refusal.world_gone"
    )
  })

  it("names the file the refusal is about", () => {
    refuse(
      refusal("DATAPACK_NAME_TAKEN", "English", { filename: "Tom & Jerry.zip" })
    )
    expect(toastError.mock.calls.map(([text]) => text)).toEqual([
      "content:_trn_worlds.refusal.name_taken Tom & Jerry.zip"
    ])
  })

  it("falls back to the core's sentence when the data lacks the file name", () => {
    refuse(refusal("DATAPACK_NAME_TAKEN", "This world already holds that file"))
    expect(toastError.mock.calls.map(([text]) => text)).toEqual([
      "This world already holds that file"
    ])
  })

  it.each([
    ["WORLD_CHANGED", "The world changed"],
    ["WORLD_CHANGED_DURING_COPY", "The world changed during the copy"],
    ["DATAPACK_CHANGED", "The data pack changed"]
  ])(
    "%s has the worlds of the instance read again, so a second try can work",
    (code, display) => {
      refuse(refusal(code, display), { instance_id: 5, world: "My World" })
      expect(invalidate.mock.calls).toEqual([
        [{ queryKey: ["instance.getWorlds", 5] }]
      ])
    }
  )

  it("leaves the world lists alone after a refusal that does not outdate them", () => {
    refuse(refusal("NOT_A_DATAPACK_VERSION", "Not a data pack"), {
      instance_id: 5
    })
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(invalidate).not.toHaveBeenCalled()
  })

  it("still leaves a code it has no sentence for to the caller", () => {
    refuse(refusal("ALREADY_IN_WORLD", "English", { filenames: ["a.zip"] }), {
      instance_id: 5
    })
    expect(toastError).not.toHaveBeenCalled()
    expect(invalidate).not.toHaveBeenCalled()
  })
})

describe("Play refused for a sign-in that could not be renewed", () => {
  const dispatchEvent = vi.fn()
  beforeEach(() => {
    toastError.mockClear()
    vi.stubGlobal("window", { dispatchEvent })
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    dispatchEvent.mockClear()
  })

  it("asks in the dialog for that instance, not in a toast", () => {
    queryClient.getMutationCache().config.onError?.(
      corePayload([
        {
          display: "The Microsoft sign-in has expired",
          code: "ACCOUNT_SIGN_IN_EXPIRED",
          data: { instance_id: 7 }
        }
      ]),
      undefined,
      undefined,
      { meta: undefined } as never,
      {} as never
    )
    expect(toastError).not.toHaveBeenCalled()
    expect(dispatchEvent).toHaveBeenCalledTimes(1)
    const event = dispatchEvent.mock.calls[0][0] as CustomEvent
    expect(event.type).toBe("gdl:sign-in-expired")
    expect(event.detail).toEqual({ instance_id: 7 })
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
