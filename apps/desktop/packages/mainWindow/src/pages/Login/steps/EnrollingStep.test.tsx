import { afterEach, describe, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { EnrollingStep } from "./EnrollingStep"

const control = vi.hoisted(() => ({
  query: undefined as any,
  flow: undefined as any,
  client: undefined as any
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      queryClient: control.client,
      client: { query: vi.fn() }
    }),
    createQuery: (options: any) => {
      control.client.getQueryCache().build(control.client, options())
      return control.query
    }
  }
}))
vi.mock("../flow/FlowContext", () => ({ useFlow: () => control.flow }))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", () => ({
  Button: (props: any) => (
    <button disabled={props.disabled} onClick={props.onClick}>
      {props.children}
    </button>
  ),
  Progress: () => <span>progress</span>,
  Skeleton: () => <span>skeleton</span>,
  Popover: (props: any) => <div>{props.children}</div>,
  PopoverContent: (props: any) => <div>{props.children}</div>,
  PopoverTrigger: (props: any) => <div>{props.children}</div>,
  CopyText: (props: any) => <span>{props.value}</span>,
  toast: { success: vi.fn(), error: vi.fn() }
}))
vi.mock("@/utils/GateAnimationRiveWrapper", () => ({
  default: () => <span>animation</span>
}))
vi.mock("qrcode", () => ({ default: { toCanvas: vi.fn(async () => {}) } }))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.restoreAllMocks()
})
function fixture(method: "browser" | "device-code" = "browser") {
  const waiting = {
    waitingForBrowser: {
      expires_at: new Date(Date.now() + 300000).toISOString(),
      auth_url: "https://synthetic.invalid",
      redirect_uri: "synthetic"
    }
  }
  const [data, setData] = createSignal<any>(waiting)
  const [error, setError] = createSignal<any>(null)
  const [updated, setUpdated] = createSignal(1)
  let revision = 1
  const calls: string[] = []
  const flow = {
    captureEnrollment: () => revision,
    ownsEnrollment: (value: number) => value === revision,
    startEnrollment: vi.fn(async (method: string) => {
      ++revision
      calls.push(`begin:${method}`)
    }),
    cancelEnrollment: vi.fn(async () => {
      ++revision
      calls.push("cancel")
    }),
    goToStep: vi.fn(async () => {}),
    finalizeEnrollment: vi.fn(async () => true),
    checkGDLAccount: vi.fn(async () => ({ type: "none" }))
  }
  const query = {
    get data() {
      return data()
    },
    get error() {
      return error()
    },
    get errorUpdatedAt() {
      return updated()
    },
    get dataUpdatedAt() {
      return updated()
    },
    refetch: vi.fn(async () => {
      setData(waiting)
      setError(null)
      setUpdated(updated() + 1)
      return { data: waiting }
    })
  }
  // This file keeps its controlled rendered callers. Installed Query lifecycle
  // and request ownership are exercised separately in queryOwnership.test.tsx.
  const entries = new Map<string, any>()
  const cache = {
    get: (hash: string) => entries.get(hash),
    build: (_client: any, options: any) => {
      const entry = {
        queryHash: options.queryHash,
        reset: () => {
          void query.refetch()
        }
      }
      entries.set(options.queryHash, entry)
      return entry
    }
  }
  control.client = {
    getQueryCache: () => cache,
    removeQueries: ({ predicate }: any) => {
      for (const [hash, entry] of entries)
        if (predicate(entry)) entries.delete(hash)
    }
  }
  control.query = query
  control.flow = flow
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <EnrollingStep step={{ type: "enrolling", method }} />,
    root
  )
  const deny = () => {
    setError(
      new Error(
        JSON.stringify({
          cause: [{ display: "access_denied: Cancelled by user" }],
          backtrace: "private trace"
        })
      )
    )
    setUpdated(updated() + 1)
  }
  return {
    root,
    flow,
    query,
    calls,
    deny,
    setError,
    setData,
    setUpdated,
    waiting
  }
}
const button = (root: HTMLElement, key: string) =>
  Array.from(root.querySelectorAll("button")).find((b) =>
    b.textContent?.includes(key)
  )!
describe("Task20 actual enrollment error presentation", () => {
  it("accepted query denial replaces waiting with details and current method retry", async () => {
    const { root, deny, flow } = fixture()
    deny()
    await Promise.resolve()
    expect(root.textContent).toContain("access_denied: Cancelled by user")
    expect(root.textContent).not.toContain("private trace")
    expect(root.textContent).not.toContain("waiting_for_browser_confirmation")
    button(root, "try_again").click()
    await Promise.resolve()
    await Promise.resolve()
    expect(flow.startEnrollment).toHaveBeenCalledExactlyOnceWith("browser")
  })
  it("denial switch cancels then begins device code through owned transition", async () => {
    const { root, deny, calls } = fixture()
    deny()
    await Promise.resolve()
    expect(root.textContent).toContain("access_denied")
    button(root, "try_device_code_instead").click()
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()
    expect(calls).toEqual(["cancel", "begin:device-code"])
  })
  it("retry clears stale error until fresh waiting data and permits a new denial", async () => {
    const { root, deny } = fixture()
    deny()
    await Promise.resolve()
    expect(root.textContent).toContain("access_denied")
    button(root, "try_again").click()
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()
    expect(root.textContent).not.toContain("access_denied")
    expect(root.textContent).toContain("waiting_for_browser_confirmation")
    deny()
    await Promise.resolve()
    expect(root.textContent).toContain("access_denied")
  })
  it("typed failure is actionable while complete and profile continuation preserve Task19", async () => {
    const { root, setData, flow } = fixture("device-code")
    // Existing EnrollmentError wire fields; this control consumes description.
    setData({
      failed: {
        errorType: "deviceCodeExpired",
        title: "Authentication Code Expired",
        description: "synthetic failure",
        recoverySteps: [],
        supportLink: ""
      }
    })
    await Promise.resolve()
    expect(root.textContent).toContain("synthetic failure")
    button(root, "try_again").click()
    await Promise.resolve()
    await Promise.resolve()
    expect(flow.startEnrollment).toHaveBeenCalledExactlyOnceWith("device-code")
    setData({
      complete: { flow_id: "owned-flow", account: { uuid: "synthetic" } }
    })
    await Promise.resolve()
    await Promise.resolve()
    expect(flow.finalizeEnrollment).toHaveBeenCalledWith(
      "owned-flow",
      expect.any(Number)
    )
    setData({
      needsProfileCreation: {
        flow_id: "profile-flow",
        access_token: "dummy",
        verification_retry: true
      }
    })
    await Promise.resolve()
    expect(flow.goToStep).toHaveBeenCalledWith({
      type: "profile-creation",
      flowId: "profile-flow",
      accessToken: "dummy",
      verificationRetry: true
    })
  })
  it("retired component retry and switch cannot mutate a replacement", async () => {
    const { root, deny, flow } = fixture()
    deny()
    await Promise.resolve()
    await flow.startEnrollment("browser")
    flow.startEnrollment.mockClear()
    flow.cancelEnrollment.mockClear()
    button(root, "try_again").click()
    button(root, "try_device_code_instead").click()
    await Promise.resolve()
    expect(flow.startEnrollment).not.toHaveBeenCalled()
    expect(flow.cancelEnrollment).not.toHaveBeenCalled()
  })
  it("late retry failure cannot overwrite a replacement", async () => {
    const { root, deny, flow } = fixture()
    deny()
    await Promise.resolve()
    let reject!: (error: unknown) => void
    flow.startEnrollment.mockImplementationOnce((_method: string) => {
      // Retire synchronously, as the actual FlowController does.
      flow.cancelEnrollment()
      return new Promise<void>((_, r) => {
        reject = r
      })
    })
    button(root, "try_again").click()
    await Promise.resolve()
    await flow.startEnrollment("device-code")
    reject(new Error("obsolete retry error"))
    await Promise.resolve()
    await Promise.resolve()
    expect(root.textContent).not.toContain("obsolete retry error")
  })
})
