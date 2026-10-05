import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal, Show, Switch, onCleanup } from "solid-js"
import {
  QueryClient,
  QueryClientProvider,
  createQuery
} from "@tanstack/solid-query"
import { EnrollingBranch, EnrollingStep } from "./EnrollingStep"
import QRCode from "qrcode"

const control = vi.hoisted(() => ({
  client: undefined as any,
  flow: undefined as any,
  query: undefined as any,
  requests: [] as (() => Promise<any>)[],
  starts: 0,
  mounts: 0,
  unmounts: 0,
  qr: vi.fn(async () => {})
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({
      queryClient: control.client,
      client: {
        query: async (_args: any, options: any) => {
          // Consume the same AbortSignal as the production rspcFetch wrapper. The
          // inert transport intentionally keeps running to test late completion.
          void options?.signal
          ++control.starts
          const request = control.requests.shift()
          if (!request) throw new Error("unplanned in-memory status query")
          return request()
        }
      }
    }),
    createQuery: (options: any) => {
      ++control.mounts
      onCleanup(() => {
        ++control.unmounts
      })
      control.query = createQuery(
        () => ({
          ...options(),
          queryFn:
            options().queryFn ??
            ((ctx: any) => {
              void ctx.signal
              ++control.starts
              const request = control.requests.shift()
              if (!request) throw new Error("unplanned in-memory status query")
              return request()
            })
        }),
        () => control.client
      )
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
vi.mock("qrcode", () => ({ default: { toCanvas: control.qr } }))
// Vite resolves the package's browser mapping to this identity in jsdom.
vi.mock("qrcode/lib/browser.js", () => ({ default: { toCanvas: control.qr } }))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  control.client?.clear()
  document.body.textContent = ""
  vi.restoreAllMocks()
})
const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 15))
const denial = (display: string) =>
  new Error(JSON.stringify({ cause: [{ display }] }))
const waiting = (flow: string) => ({
  waitingForBrowser: {
    expires_at: new Date(Date.now() + 300000).toISOString(),
    auth_url: `https://synthetic.invalid/${flow}`,
    redirect_uri: "synthetic"
  }
})
const complete = (flow: string) => ({
  complete: { flow_id: flow, account: { uuid: "synthetic" } }
})
const profile = (flow: string) => ({
  needsProfileCreation: {
    flow_id: flow,
    access_token: "dummy",
    verification_retry: true
  }
})
function deferred() {
  let resolve!: (value: any) => void
  let reject!: (value: any) => void
  const promise = new Promise<any>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={control.client}>
        <EnrollingStep step={{ type: "enrolling", method: "browser" }} />
      </QueryClientProvider>
    ),
    root
  )
  return root
}
async function fixture() {
  let revision = 1
  control.starts = 0
  control.requests = [() => Promise.reject(denial("initial A denial"))]
  control.client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, networkMode: "always", gcTime: 0 }
    }
  })
  control.flow = {
    captureEnrollment: () => revision,
    ownsEnrollment: (value: number) => value === revision,
    startEnrollment: vi.fn(async () => {
      ++revision
    }),
    cancelEnrollment: vi.fn(async () => {
      ++revision
    }),
    goToStep: vi.fn(async () => {}),
    finalizeEnrollment: vi.fn(async () => true),
    checkGDLAccount: vi.fn(async () => ({ type: "none" }))
  }
  const root = mount()
  await settle()
  expect(root.textContent).toContain("initial A denial")
  expect(control.query.data).toBeUndefined()
  return { root, revision: () => revision }
}
function retry(root: HTMLElement) {
  const button = Array.from(root.querySelectorAll("button")).find((item) =>
    item.textContent?.includes("try_again")
  )
  expect(button).toBeDefined()
  button!.click()
}
const refetch = () =>
  control.client.refetchQueries({ queryKey: ["account.enroll.getStatus"] })

const deviceCode = (value: string) => ({
  pollingCode: {
    expiresAt: new Date(Date.now() + 300000).toISOString(),
    verificationUri: "https://synthetic.invalid/device",
    userCode: value
  }
})
async function branchFixture(method: "browser" | "device-code") {
  // This consumes the same production Match branch as AuthFlow. The outer
  // phase/step selector mirrors its traced state contract without page startup.
  expect(QRCode.toCanvas).toBe(control.qr)
  control.qr.mockClear()
  control.starts = 0
  control.mounts = 0
  control.unmounts = 0
  control.requests = [
    method === "browser"
      ? () => Promise.reject(denial("branch browser denial"))
      : () => Promise.resolve(deviceCode("FRESH-DEVICE"))
  ]
  control.client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, networkMode: "always", gcTime: 0 }
    }
  })
  let revision = 1
  const [state, setState] = createSignal<any>({
    phase: "content",
    step: { type: "enrolling", method }
  })
  control.flow = {
    captureEnrollment: () => revision,
    ownsEnrollment: (value: number) => value === revision,
    startEnrollment: vi.fn(async () => {
      ++revision
    }),
    cancelEnrollment: vi.fn(async () => {
      ++revision
    }),
    goToStep: vi.fn(async (step: any) => {
      const origin = revision
      await new Promise<void>((resolve) => queueMicrotask(resolve))
      if (origin === revision) setState({ phase: "content", step })
    }),
    finalizeEnrollment: vi.fn(async () => true),
    checkGDLAccount: vi.fn(async () => ({ type: "none" }))
  }
  const currentStep = () =>
    state().phase === "content"
      ? state().step
      : state().phase === "transitioning"
        ? state().to
        : null
  const enrolling = () =>
    currentStep()?.type === "enrolling" ? currentStep() : null
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <QueryClientProvider client={control.client}>
        <Show
          when={
            state().phase === "content" || state().phase === "transitioning"
          }
        >
          <Switch>
            <EnrollingBranch step={enrolling()} />
          </Switch>
        </Show>
      </QueryClientProvider>
    ),
    root
  )
  await settle()
  return { root, state, setState, revision: () => revision }
}

it("production enrolling branch keeps fresh device denial and Retry usable", async () => {
  const { root, revision } = await branchFixture("device-code")
  expect(root.textContent).toContain("FRESH-DEVICE")
  expect(control.qr).toHaveBeenCalledWith(
    expect.any(HTMLCanvasElement),
    "https://synthetic.invalid/device",
    expect.any(Object)
  )
  control.requests.push(() => Promise.reject(denial("branch device denial")))
  await refetch()
  await settle()
  expect(root.textContent).toContain("branch device denial")
  control.requests.push(() => Promise.resolve(deviceCode("DEVICE-RETRY")))
  retry(root)
  await settle()
  expect(revision()).toBe(2)
  expect(root.textContent).toContain("DEVICE-RETRY")
  expect(control.flow.startEnrollment).toHaveBeenCalledExactlyOnceWith(
    "device-code"
  )
  expect(control.mounts).toBe(1)
})
it("production enrolling branch preserves owner across phases with the same step identity", async () => {
  const { root, state, setState } = await branchFixture("device-code")
  const step = state().step
  setState({ phase: "transitioning", to: step, direction: "forward" })
  await settle()
  setState({ phase: "content", step })
  await settle()
  expect(root.textContent).toContain("FRESH-DEVICE")
  expect(control.mounts).toBe(1)
  expect(control.unmounts).toBe(0)
  expect(control.starts).toBe(1)
})
for (const outcome of ["complete", "profile"] as const) {
  it(`production enrolling branch replaces switched owner and retains device Retry/${outcome}`, async () => {
    const { root, revision, state } = await branchFixture("browser")
    expect(root.textContent).toContain("branch browser denial")
    control.requests.push(() => Promise.resolve(deviceCode("SWITCHED-DEVICE")))
    Array.from(root.querySelectorAll("button"))
      .find((button) =>
        button.textContent?.includes("try_device_code_instead")
      )!
      .click()
    await settle()
    expect(state()).toEqual({
      phase: "content",
      step: { type: "enrolling", method: "device-code" }
    })
    expect(revision()).toBe(3)
    expect(root.textContent).toContain("SWITCHED-DEVICE")
    expect(root.textContent).not.toContain("branch browser denial")
    expect(control.starts).toBe(2)
    expect(control.mounts).toBe(2)
    expect(control.unmounts).toBe(1)
    expect(control.flow.cancelEnrollment).toHaveBeenCalledOnce()
    expect(control.qr).toHaveBeenCalledWith(
      expect.any(HTMLCanvasElement),
      "https://synthetic.invalid/device",
      expect.any(Object)
    )
    control.requests.push(() =>
      Promise.reject(denial("switched device denial"))
    )
    await refetch()
    await settle()
    expect(root.textContent).toContain("switched device denial")
    control.requests.push(() => Promise.resolve(deviceCode("SWITCHED-RETRY")))
    retry(root)
    await settle()
    expect(revision()).toBe(4)
    expect(root.textContent).toContain("SWITCHED-RETRY")
    expect(control.flow.startEnrollment.mock.calls).toEqual([
      ["device-code"],
      ["device-code"]
    ])
    control.requests.push(() =>
      Promise.resolve(
        outcome === "complete"
          ? complete("device-flow")
          : profile("device-flow")
      )
    )
    await refetch()
    await settle()
    if (outcome === "complete")
      expect(control.flow.finalizeEnrollment).toHaveBeenCalledExactlyOnceWith(
        "device-flow",
        4
      )
    else
      expect(control.flow.goToStep).toHaveBeenCalledWith({
        type: "profile-creation",
        flowId: "device-flow",
        accessToken: "dummy",
        verificationRetry: true
      })
  })
}

it("healthy retry fetches B and presents B current denial", async () => {
  const { root, revision } = await fixture()
  control.requests.push(() => Promise.resolve(waiting("B")))
  retry(root)
  await settle()
  expect(revision()).toBe(2)
  expect(control.starts).toBe(2)
  expect(control.query.data.waitingForBrowser.auth_url).toBe(
    "https://synthetic.invalid/B"
  )
  control.requests.push(() => Promise.reject(denial("current B denial")))
  await refetch()
  await settle()
  expect(root.textContent).toContain("current B denial")
})
for (const outcome of ["error", "waiting", "complete", "profile"] as const) {
  it(`pending A ${outcome} cannot become Retry B status`, async () => {
    const { root, revision } = await fixture()
    const old = deferred()
    control.requests.push(() => old.promise)
    const oldPoll = refetch()
    await settle()
    expect(control.starts).toBe(2)
    control.requests.push(() => Promise.resolve(waiting("B")))
    retry(root)
    await settle()
    expect(revision()).toBe(2)
    if (outcome === "error") old.reject(denial("obsolete A denial"))
    else
      old.resolve(
        outcome === "waiting"
          ? waiting("A")
          : outcome === "complete"
            ? complete("A")
            : profile("A")
      )
    await oldPoll
    await settle()
    expect(root.textContent).not.toContain("obsolete A denial")
    expect(control.query.data?.waitingForBrowser?.auth_url).toBe(
      "https://synthetic.invalid/B"
    )
    expect(control.starts).toBe(3)
    expect(control.flow.finalizeEnrollment).not.toHaveBeenCalled()
    expect(control.flow.goToStep).not.toHaveBeenCalled()
  })
}
for (const outcome of ["complete", "profile"] as const) {
  it(`cached A ${outcome} is not dispatched during pending B begin`, async () => {
    const { root } = await fixture()
    const begin = deferred()
    control.flow.startEnrollment.mockImplementationOnce(async () => {
      control.flow.cancelEnrollment()
      await begin.promise
    })
    retry(root)
    control.client.setQueryData(
      control.query.queryKey ?? ["account.enroll.getStatus"],
      outcome === "complete" ? complete("A") : profile("A")
    )
    // setQueryData is a real cache publication while begin remains pending.
    for (const query of control.client
      .getQueryCache()
      .findAll({ queryKey: ["account.enroll.getStatus"] })) {
      query.setData(outcome === "complete" ? complete("A") : profile("A"))
    }
    await settle()
    expect(control.flow.finalizeEnrollment).not.toHaveBeenCalled()
    expect(control.flow.goToStep).not.toHaveBeenCalled()
    control.requests.push(() => Promise.resolve(waiting("B")))
    begin.resolve(undefined)
    await settle()
    expect(control.query.data?.waitingForBrowser?.auth_url).toBe(
      "https://synthetic.invalid/B"
    )
  })
}

for (const replacement of ["same controller", "new controller"] as const) {
  it(`late begin cannot reset ${replacement} replacement with the same revision`, async () => {
    const { root } = await fixture()
    const begin = deferred()
    const oldFlow = control.flow
    oldFlow.startEnrollment.mockImplementationOnce(async () => {
      oldFlow.cancelEnrollment()
      await begin.promise
    })
    retry(root)
    await settle()
    dispose!()
    dispose = undefined
    root.remove()
    if (replacement === "new controller")
      control.flow = {
        ...oldFlow,
        captureEnrollment: () => 2,
        ownsEnrollment: (revision: number) => revision === 2,
        startEnrollment: vi.fn(),
        cancelEnrollment: vi.fn(),
        goToStep: vi.fn(),
        finalizeEnrollment: vi.fn(async () => true)
      }
    control.requests.push(() => Promise.resolve(waiting("replacement")))
    const currentRoot = mount()
    await settle()
    const currentQuery = control.client
      .getQueryCache()
      .findAll({ queryKey: ["account.enroll.getStatus"] })
      .find(
        (query: any) =>
          query.state.data?.waitingForBrowser?.auth_url ===
          "https://synthetic.invalid/replacement"
      )
    expect(currentQuery).toBeDefined()
    begin.resolve(undefined)
    await settle()
    expect(control.client.getQueryCache().get(currentQuery.queryHash)).toBe(
      currentQuery
    )
    expect(control.query.data.waitingForBrowser.auth_url).toBe(
      "https://synthetic.invalid/replacement"
    )
    control.requests.push(() =>
      Promise.reject(denial("replacement current denial"))
    )
    await refetch()
    await settle()
    expect(currentRoot.textContent).toContain("replacement current denial")
  })
}
for (const outcome of ["complete", "profile"] as const) {
  it(`current B ${outcome} dispatch retains backend flow identity`, async () => {
    const { root } = await fixture()
    control.requests.push(() =>
      Promise.resolve(outcome === "complete" ? complete("B") : profile("B"))
    )
    retry(root)
    await settle()
    if (outcome === "complete")
      expect(control.flow.finalizeEnrollment).toHaveBeenCalledExactlyOnceWith(
        "B",
        2
      )
    else
      expect(control.flow.goToStep).toHaveBeenCalledWith({
        type: "profile-creation",
        flowId: "B",
        accessToken: "dummy",
        verificationRetry: true
      })
  })
}
it("installed current denial switch preserves cancel then device-code begin", async () => {
  const { root } = await fixture()
  Array.from(root.querySelectorAll("button"))
    .find((button) => button.textContent?.includes("try_device_code_instead"))!
    .click()
  await settle()
  expect(control.flow.cancelEnrollment).toHaveBeenCalledOnce()
  expect(control.flow.startEnrollment).toHaveBeenCalledExactlyOnceWith(
    "device-code"
  )
  expect(control.flow.goToStep).toHaveBeenCalledWith({
    type: "enrolling",
    method: "device-code"
  })
})
it("disposed pending begin cannot launch another retained-cache status request", async () => {
  const { root } = await fixture()
  const begin = deferred()
  control.client.setDefaultOptions({
    queries: { retry: false, gcTime: Infinity, networkMode: "always" }
  })
  control.flow.startEnrollment.mockImplementationOnce(async () => {
    control.flow.cancelEnrollment()
    await begin.promise
  })
  retry(root)
  await settle()
  dispose!()
  dispose = undefined
  root.remove()
  control.requests.push(() => Promise.resolve(waiting("unmounted")))
  begin.resolve(undefined)
  await settle()
  expect(control.starts).toBe(1)
})
it("stale component cleanup and clicks cannot retire a newer same-revision Query", async () => {
  const { root } = await fixture()
  const oldDispose = dispose!
  control.requests.push(() => Promise.resolve(waiting("replacement")))
  const currentRoot = mount()
  await settle()
  const currentQuery = control.client
    .getQueryCache()
    .findAll({ queryKey: ["account.enroll.getStatus"] })
    .find(
      (query: any) =>
        query.state.data?.waitingForBrowser?.auth_url ===
        "https://synthetic.invalid/replacement"
    )
  expect(currentQuery).toBeDefined()
  retry(root)
  Array.from(root.querySelectorAll("button"))
    .find((button) => button.textContent?.includes("try_device_code_instead"))!
    .click()
  expect(control.flow.startEnrollment).not.toHaveBeenCalled()
  expect(control.flow.cancelEnrollment).not.toHaveBeenCalled()
  oldDispose()
  root.remove()
  await settle()
  expect(control.client.getQueryCache().get(currentQuery.queryHash)).toBe(
    currentQuery
  )
  control.requests.push(() => Promise.reject(denial("surviving B denial")))
  await refetch()
  await settle()
  expect(currentRoot.textContent).toContain("surviving B denial")
})
for (const outcome of ["complete", "profile"] as const) {
  it(`remount B rejects cached A ${outcome} before B status`, async () => {
    const { root } = await fixture()
    dispose!()
    dispose = undefined
    root.remove()
    await control.flow.startEnrollment("browser")
    control.client.setQueryData(
      ["account.enroll.getStatus"],
      outcome === "complete" ? complete("A") : profile("A")
    )
    const current = deferred()
    control.requests.push(() => current.promise)
    mount()
    await settle()
    expect(control.flow.finalizeEnrollment).not.toHaveBeenCalled()
    expect(control.flow.goToStep).not.toHaveBeenCalled()
    current.resolve(waiting("B"))
    await settle()
    expect(control.query.data?.waitingForBrowser?.auth_url).toBe(
      "https://synthetic.invalid/B"
    )
  })
}
