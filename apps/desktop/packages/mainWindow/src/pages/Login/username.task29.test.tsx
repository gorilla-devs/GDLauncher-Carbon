import { afterEach, expect, it, vi } from "vitest"
import { createSignal, Match } from "solid-js"
import { render } from "solid-js/web"
import { deferred } from "../../task29.fixture"
import { AuthFlow } from "./AuthFlow"

const control = vi.hoisted(() => ({ flow: undefined as any }))
vi.mock("@gd/ui", async () => import("../../task29.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [{}],
  useLocation: () => ({ pathname: "/" })
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    accounts: { data: [] },
    settings: { data: {} },
    currentlySelectedAccountUuid: { data: null }
  })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({}),
    createQuery: () => ({ data: false }),
    createMutation: () => ({})
  }
}))
vi.mock("./flow/FlowContext", () => ({
  useFlow: () => control.flow,
  FlowProvider: (props: any) => <div>{props.children}</div>
}))
vi.mock("./animations/AnimationContext", () => ({
  AnimationProvider: (props: any) => <div>{props.children}</div>,
  useAnimations: () => ({
    registerRefs: vi.fn(),
    sidebar: { slideIn: vi.fn() },
    backButton: { show: vi.fn(), hide: vi.fn() },
    skipButton: { show: vi.fn(), hide: vi.fn() }
  })
}))
vi.mock("./components/StepContainer", () => ({
  StepContainer: (props: any) => <div>{props.children}</div>
}))
vi.mock("./steps/WelcomeStep", () => ({ WelcomeStep: () => null }))
vi.mock("./steps/TermsStep", () => ({ TermsStep: () => null }))
vi.mock("./steps/AuthMethodStep", () => ({ AuthMethodStep: () => null }))
vi.mock("./steps/EnrollingStep", () => ({
  EnrollingBranch: () => (
    <Match when={false}>
      <span />
    </Match>
  )
}))
vi.mock("./steps/ProfileCreationStep", () => ({
  ProfileCreationStep: (props: any) => (
    <div>
      <input
        value={props.username}
        onInput={(event) => props.onUsernameChange(event.currentTarget.value)}
      />
      <span data-testid="availability">{props.available}</span>
      <span data-testid="checking">{String(props.checking)}</span>
      <span data-testid="error">{props.error}</span>
    </div>
  )
}))
vi.mock("./steps/GdlAccountStep", () => ({ GdlAccountStep: () => null }))
vi.mock("./steps/GdlAccountFormStep", () => ({
  GdlAccountFormStep: () => null
}))
vi.mock("./steps/GdlAccountVerificationStep", () => ({
  GdlAccountVerificationStep: () => null
}))
vi.mock("./steps/ErrorStep", () => ({ ErrorStep: () => null }))
vi.mock("@/utils/occasions", () => ({ getCurrentOccasion: () => null }))
vi.mock("@/utils/logos", () => ({ wideLogoUrl: "synthetic" }))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.useRealTimers()
  vi.restoreAllMocks()
})
function fixture() {
  vi.useFakeTimers()
  let revision = 1
  const [state, setState] = createSignal<any>({
    phase: "content",
    step: {
      type: "profile-creation",
      accessToken: "token-A",
      flowId: "flow-A",
      verificationRetry: false
    }
  })
  const requests: any[] = []
  control.flow = {
    state,
    nextDirection: () => undefined,
    data: { accounts: [] },
    captureEnrollment: () => revision,
    ownsEnrollment: (value: number) => value === revision,
    cleanup: () => {
      ++revision
    },
    checkUsernameAvailability: vi.fn(() => {
      const request = deferred()
      requests.push(request)
      return request.promise
    }),
    createProfile: vi.fn(async () => "retryable")
  }
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <AuthFlow />, root)
  const edit = (value: string) => {
    const input = root.querySelector("input")!
    input.value = value
    input.dispatchEvent(new Event("input", { bubbles: true }))
  }
  const create = () =>
    Array.from(root.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("create_profile")
    )!
  const start = async (name: string) => {
    edit(name)
    await vi.advanceTimersByTimeAsync(500)
  }
  return {
    root,
    start,
    edit,
    requests,
    create,
    setState,
    retire: () => {
      ++revision
    }
  }
}
it("0252 Beta stays available after old Alpha Taken completes", async () => {
  const { root, start, requests, create } = fixture()
  await start("Alpha_01")
  await start("Beta_02")
  requests[1].resolve("available")
  await Promise.resolve()
  expect(create().disabled).toBe(false)
  requests[0].resolve("taken")
  await Promise.resolve()
  expect(root.querySelector('[data-testid="availability"]')!.textContent).toBe(
    "available"
  )
  expect(create().disabled).toBe(false)
})
it("0252 old catch/finally cannot stop the newer pending check", async () => {
  const { root, start, requests, create } = fixture()
  await start("Alpha_01")
  await start("Beta_02")
  requests[0].reject(new Error("old failure"))
  await Promise.resolve()
  await Promise.resolve()
  expect(root.querySelector('[data-testid="checking"]')!.textContent).toBe(
    "true"
  )
  expect(root.querySelector('[data-testid="error"]')!.textContent).toBe("")
  expect(create().disabled).toBe(true)
  requests[1].resolve("available")
  await Promise.resolve()
  expect(create().disabled).toBe(false)
})
it("0252 edit before debounce, invalid edit and exact token/step replacement revoke old proof", async () => {
  const { start, edit, requests, create, setState } = fixture()
  edit("Alpha_01")
  edit("Beta_02")
  await vi.advanceTimersByTimeAsync(500)
  expect(
    control.flow.checkUsernameAvailability
  ).toHaveBeenCalledExactlyOnceWith("token-A", "Beta_02")
  requests[0].resolve("available")
  await Promise.resolve()
  expect(create().disabled).toBe(false)
  setState({
    phase: "content",
    step: {
      type: "profile-creation",
      accessToken: "token-B",
      flowId: "flow-B",
      verificationRetry: false
    }
  })
  expect(create().disabled).toBe(true)
  await vi.advanceTimersByTimeAsync(500)
  edit("!")
  requests[1].resolve("available")
  await Promise.resolve()
  expect(create().disabled).toBe(true)
  await start("Gamma_03")
  requests[2].resolve("available")
  await Promise.resolve()
  expect(create().disabled).toBe(false)
  create().click()
  await Promise.resolve()
  expect(control.flow.createProfile).toHaveBeenCalledWith(
    "flow-B",
    "token-B",
    "Gamma_03",
    false
  )
})
it("0252 step exit and disposal ignore late check completions", async () => {
  const { start, requests, setState } = fixture()
  await start("Alpha_01")
  setState({ phase: "content", step: { type: "auth-method" } })
  requests[0].resolve("available")
  await Promise.resolve()
  setState({
    phase: "content",
    step: {
      type: "profile-creation",
      accessToken: "token-B",
      flowId: "flow-B",
      verificationRetry: false
    }
  })
  await vi.advanceTimersByTimeAsync(500)
  dispose!()
  dispose = undefined
  requests[1].resolve("available")
  await Promise.resolve()
  expect(control.flow.createProfile).not.toHaveBeenCalled()
})
it("0252 same name and token in a replacement step cannot adopt an old result", async () => {
  const { start, requests, create, setState } = fixture()
  await start("Alpha_01")
  setState({
    phase: "content",
    step: {
      type: "profile-creation",
      accessToken: "token-A",
      flowId: "flow-B",
      verificationRetry: false
    }
  })
  await vi.advanceTimersByTimeAsync(500)
  requests[1].resolve("available")
  await Promise.resolve()
  expect(create().disabled).toBe(false)
  requests[0].resolve("taken")
  await Promise.resolve()
  expect(create().disabled).toBe(false)
})
