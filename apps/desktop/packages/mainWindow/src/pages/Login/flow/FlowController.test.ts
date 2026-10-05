import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { FlowControllerImpl } from "./FlowController"
import {
  completeEnrollment,
  switchEnrollment,
  leaveEnrollment
} from "./enrollmentTransitions"

beforeEach(() => vi.stubGlobal("document", {}))
afterEach(() => vi.unstubAllGlobals())

function deferred<T = void>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((r) => {
    resolve = r
  })
  return { promise, resolve }
}
function fixture(create: () => Promise<unknown>) {
  const resume = vi.fn(async () => "complete" as const)
  const refetch = vi.fn(async () => ({ data: [] }))
  const mutateAsync = vi.fn(async () => null)
  const finalize = vi.fn(async () => "complete" as "complete" | "obsolete")
  const cancel = vi.fn(async () => null)
  const begin = vi.fn(async () => null)
  const beginBrowser = vi.fn(async () => null)
  const query = vi.fn(async () => "old-a")
  const flow = new FlowControllerImpl(
    {
      activeUuid: "before",
      accounts: [],
      isFirstLaunch: false,
      termsAndPrivacyAccepted: true,
      gdlAccountId: null,
      reducedMotion: true,
      isAddingMicrosoftFromSettings: true,
      isAddingGdlFromSettings: false,
      shouldSetupGdlAfterAuth: false,
      returnPath: null,
      currentOccasion: null
    },
    {
      rspcContext: {
        queryClient: { refetchQueries: refetch },
        client: { query, mutation: finalize }
      },
      settingsMutation: { mutateAsync },
      saveGdlAccountMutation: { mutateAsync },
      enrollBeginMutation: { mutateAsync: begin },
      enrollBeginBrowserMutation: { mutateAsync: beginBrowser },
      enrollCancelMutation: { mutateAsync: cancel },
      usernameAvailabilityMutation: { mutateAsync },
      createProfileMutation: { mutateAsync: create },
      enrollResumeMutation: { mutateAsync: resume },
      accountsQuery: { refetch },
      gdlAccountQuery: { refetch }
    } as any
  )
  return { flow, resume, refetch, finalize, cancel, begin, beginBrowser, query }
}
describe("Task19 profile continuation ownership", () => {
  it("does not resume a replacement after cancelled profile creation", async () => {
    const creation = deferred()
    const { flow, resume, refetch } = fixture(() => creation.promise)
    const old = flow.createProfile("flow-a", "dummy-token", "Example")
    await flow.cancelEnrollment()
    await flow.startEnrollment("device-code")
    creation.resolve()
    await old
    expect(resume).not.toHaveBeenCalled()
    expect(refetch).not.toHaveBeenCalled()
    expect(flow.data.activeUuid).toBe("before")
    flow.cleanup()
  })
  it("creation failure never starts verification", async () => {
    const { flow, resume } = fixture(async () => {
      throw new Error("synthetic creation failure")
    })
    await expect(
      flow.createProfile("flow-a", "dummy-token", "Example")
    ).rejects.toThrow("creation failure")
    expect(resume).not.toHaveBeenCalled()
    flow.cleanup()
  })
})

it("verification Retry does not repeat successful creation", async () => {
  const create = vi.fn(async () => null)
  const { flow, resume } = fixture(create)
  resume.mockResolvedValueOnce("retryable" as any)
  expect(await flow.createProfile("flow-a", "dummy-token", "Example")).toBe(
    "retryable"
  )
  expect(await flow.createProfile("flow-a", "dummy-token", "Example")).toBe(
    "complete"
  )
  expect(create).toHaveBeenCalledTimes(1)
  expect(resume).toHaveBeenNthCalledWith(1, "flow-a")
  expect(resume).toHaveBeenNthCalledWith(2, "flow-a")
  flow.cleanup()
})
it("resumed retry status invokes verification only", async () => {
  const create = vi.fn(async () => null)
  const { flow, resume } = fixture(create)
  expect(
    await flow.createProfile("flow-a", "dummy-token", "Example", true)
  ).toBe("complete")
  expect(create).not.toHaveBeenCalled()
  expect(resume).toHaveBeenCalledWith("flow-a")
  flow.cleanup()
})
it("cancel during refetch preserves replacement local data", async () => {
  const pending = deferred<any>()
  const { flow, refetch } = fixture(async () => null)
  refetch.mockImplementationOnce(() => pending.promise)
  const old = flow.createProfile("flow-a", "dummy-token", "Example")
  await vi.waitFor(() => expect(refetch).toHaveBeenCalled())
  await flow.cancelEnrollment()
  flow.data.activeUuid = "replacement-b"
  pending.resolve({ data: [] })
  expect(await old).toBe("stale")
  expect(flow.data.activeUuid).toBe("replacement-b")
  flow.cleanup()
})
it("cancel during final account query preserves replacement local data", async () => {
  const pending = deferred<any>()
  const { flow, refetch } = fixture(async () => null)
  refetch.mockImplementationOnce(async () => ({ data: [] }))
  refetch.mockImplementationOnce(async () => ({ data: [] }))
  refetch.mockImplementationOnce(() => pending.promise)
  const old = flow.createProfile("flow-a", "dummy-token", "Example")
  await vi.waitFor(() => expect(refetch).toHaveBeenCalledTimes(3))
  await flow.cancelEnrollment()
  flow.data.activeUuid = "replacement-b"
  pending.resolve({ data: [{ uuid: "old-a" }] })
  expect(await old).toBe("stale")
  expect(flow.data.activeUuid).toBe("replacement-b")
  expect(flow.data.accounts).toEqual([])
  flow.cleanup()
})

for (const stage of ["finalize", "refetch", "account-list"] as const) {
  it(`round1 stale ${stage} cannot update or advance replacement enrollment`, async () => {
    const pending = deferred<any>()
    const { flow, finalize, refetch } = fixture(async () => null)
    if (stage === "finalize")
      finalize.mockImplementationOnce(() => pending.promise)
    else if (stage === "refetch")
      refetch.mockImplementationOnce(() => pending.promise)
    else {
      refetch.mockImplementationOnce(async () => ({ data: [] }))
      refetch.mockImplementationOnce(async () => ({ data: [] }))
      refetch.mockImplementationOnce(() => pending.promise)
    }
    const check = vi
      .spyOn(flow, "checkGDLAccount")
      .mockResolvedValue({ type: "none" })
    const old = completeEnrollment(flow, "flow-a")
    await vi.waitFor(() =>
      expect(stage === "finalize" ? finalize : refetch).toHaveBeenCalledTimes(
        stage === "account-list" ? 3 : stage === "refetch" ? 2 : 1
      )
    )
    await leaveEnrollment(flow)
    await flow.startEnrollment("browser")
    await flow.goToStep({ type: "enrolling", method: "browser" })
    flow.data.activeUuid = "replacement-b"
    pending.resolve(stage === "finalize" ? null : { data: [{ uuid: "old-a" }] })
    await old
    expect(flow.data.activeUuid).toBe("replacement-b")
    expect(flow.data.accounts).toEqual([])
    expect(check).not.toHaveBeenCalled()
    expect(flow.state()).toMatchObject({
      phase: "content",
      step: { type: "enrolling", method: "browser" }
    })
    flow.cleanup()
  })
}
it("round1 stale finalize failure cannot navigate replacement to error", async () => {
  let reject!: (reason: unknown) => void
  const pending = new Promise<"complete">((_, r) => {
    reject = r
  })
  const { flow, finalize } = fixture(async () => null)
  finalize.mockImplementationOnce(() => pending)
  const warning = vi.spyOn(console, "error").mockImplementation(() => {})
  try {
    const old = completeEnrollment(flow, "flow-a")
    await vi.waitFor(() => expect(finalize).toHaveBeenCalled())
    await leaveEnrollment(flow)
    await flow.startEnrollment("browser")
    await flow.goToStep({ type: "enrolling", method: "browser" })
    reject(new Error("synthetic old finalize failure"))
    await old
    expect(flow.state()).toMatchObject({
      step: { type: "enrolling", method: "browser" }
    })
    expect(warning).not.toHaveBeenCalled()
  } finally {
    warning.mockRestore()
    flow.cleanup()
  }
})
it("round1 delayed switch cancellation cannot start after leaving and beginning B", async () => {
  const pending = deferred<null>()
  const { flow, cancel, begin, beginBrowser } = fixture(async () => null)
  cancel.mockImplementationOnce(() => pending.promise)
  const loading = vi.fn()
  const old = switchEnrollment(flow, loading)
  await vi.waitFor(() => expect(cancel).toHaveBeenCalled())
  await leaveEnrollment(flow)
  const replacement = flow.startEnrollment("browser")
  pending.resolve(null)
  await replacement
  await old
  expect(beginBrowser).toHaveBeenCalledTimes(1)
  expect(begin).not.toHaveBeenCalled()
  expect(loading.mock.calls).toEqual([[true]])
  flow.cleanup()
})
it("round1 leaving enrolling retires immediately and cancels backend", async () => {
  const pending = deferred<null>()
  const { flow, cancel } = fixture(async () => null)
  cancel.mockImplementationOnce(() => pending.promise)
  const before = flow.captureEnrollment()
  await leaveEnrollment(flow)
  expect(flow.ownsEnrollment(before)).toBe(false)
  await vi.waitFor(() => expect(cancel).toHaveBeenCalledTimes(1))
  pending.resolve(null)
  flow.cleanup()
})
it("round1 current completion binds its flow and advances normally", async () => {
  const { flow, finalize } = fixture(async () => null)
  vi.spyOn(flow, "checkGDLAccount").mockResolvedValue({ type: "none" })
  await completeEnrollment(flow, "flow-a")
  expect(finalize).toHaveBeenCalledWith(["account.enroll.finalize", "flow-a"])
  expect(flow.data.activeUuid).toBe("old-a")
  expect(flow.state()).toMatchObject({
    step: { type: "gdl-account", gdlAccount: { type: "none" } }
  })
  flow.cleanup()
})
it("round1 current switch cancels then begins and clears loading", async () => {
  const { flow, cancel, begin } = fixture(async () => null)
  const loading = vi.fn()
  await switchEnrollment(flow, loading)
  expect(cancel).toHaveBeenCalledTimes(1)
  expect(begin).toHaveBeenCalledTimes(1)
  expect(cancel.mock.invocationCallOrder[0]).toBeLessThan(
    begin.mock.invocationCallOrder[0]
  )
  expect(flow.state()).toMatchObject({
    step: { type: "enrolling", method: "device-code" }
  })
  expect(loading.mock.calls).toEqual([[true], [false]])
  flow.cleanup()
})
it("round1 cancellation during GDL check cannot advance replacement", async () => {
  const pending = deferred<any>()
  const { flow } = fixture(async () => null)
  const check = vi
    .spyOn(flow, "checkGDLAccount")
    .mockImplementation(() => pending.promise)
  const old = completeEnrollment(flow, "flow-a")
  await vi.waitFor(() => expect(check).toHaveBeenCalled())
  await leaveEnrollment(flow)
  await flow.startEnrollment("browser")
  await flow.goToStep({ type: "enrolling", method: "browser" })
  pending.resolve({ type: "none" })
  await old
  expect(flow.state()).toMatchObject({
    step: { type: "enrolling", method: "browser" }
  })
  flow.cleanup()
})
it("round1 an already obsolete caller cannot issue finalize", async () => {
  const { flow, finalize } = fixture(async () => null)
  const old = flow.captureEnrollment()
  await flow.cancelEnrollment()
  expect(await flow.finalizeEnrollment("flow-a", old)).toBe(false)
  expect(finalize).not.toHaveBeenCalled()
  flow.cleanup()
})
it("round1 cached obsolete Complete event cannot adopt replacement ownership", async () => {
  const { flow, finalize, refetch } = fixture(async () => null)
  await flow.startEnrollment("browser")
  await flow.goToStep({ type: "enrolling", method: "browser" })
  flow.data.activeUuid = "replacement-b"
  finalize.mockResolvedValueOnce("obsolete" as any)
  const check = vi
    .spyOn(flow, "checkGDLAccount")
    .mockResolvedValue({ type: "none" })
  await completeEnrollment(flow, "cached-flow-a")
  expect(refetch).not.toHaveBeenCalled()
  expect(check).not.toHaveBeenCalled()
  expect(flow.data.activeUuid).toBe("replacement-b")
  expect(flow.data.error).toBeNull()
  expect(flow.state()).toMatchObject({
    step: { type: "enrolling", method: "browser" }
  })
  flow.cleanup()
})
it("round1 current finalize failure remains actionable", async () => {
  const { flow, finalize } = fixture(async () => null)
  finalize.mockRejectedValueOnce(new Error("synthetic current write failure"))
  const check = vi
    .spyOn(flow, "checkGDLAccount")
    .mockResolvedValue({ type: "none" })
  const warning = vi.spyOn(console, "error").mockImplementation(() => {})
  try {
    await completeEnrollment(flow, "current-flow")
    expect(check).not.toHaveBeenCalled()
    expect(flow.state()).toMatchObject({
      step: {
        type: "error",
        canRetry: true,
        message: "Failed to save account. Please try again."
      }
    })
    expect(warning).toHaveBeenCalledTimes(1)
    expect(warning).toHaveBeenCalledWith(
      "[EnrollingStep] Failed to finalize enrollment:",
      expect.objectContaining({ message: "synthetic current write failure" })
    )
  } finally {
    warning.mockRestore()
    flow.cleanup()
  }
})
