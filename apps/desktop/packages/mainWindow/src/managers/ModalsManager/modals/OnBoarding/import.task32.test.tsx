import { beforeEach, afterEach, it, expect, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import SingleEntity, {
  instances,
  globalInstances,
  setInstances
} from "./SingleEntity"
import SingleCheckBox from "./SingleCheckBox"
import {
  button,
  deferred,
  input,
  pointerGesture,
  settle
} from "@/task32.fixture"
const c = vi.hoisted(() => ({
  scan: vi.fn(),
  submit: vi.fn(),
  cancel: vi.fn(),
  close: vi.fn(),
  invalidate: vi.fn(),
  status: (() => undefined) as () => any,
  defaultPath: (() => "/synthetic/A") as () => string
}))
vi.mock("@gd/ui", async () => import("@/task32.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/i18n/helpers", () => ({
  getImportErrorTranslationKey: (x: string) => x
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { invalidateQueries: c.invalidate },
  rspc: {
    createQuery: (f: any) => ({
      get data() {
        return f().queryKey[0] === "instance.getImportScanStatus"
          ? c.status()
          : c.defaultPath()
      }
    }),
    createMutation: (f: any) => ({
      mutateAsync: (v: any) =>
        f().mutationKey[0] === "instance.setImportScanTarget"
          ? c.scan(v)
          : f().mutationKey[0] === "instance.importInstance"
            ? c.submit(v)
            : c.cancel(v)
    })
  }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
let dispose: (() => void) | undefined, setStatus: any
beforeEach(() => {
  vi.clearAllMocks()
  c.scan.mockReset().mockResolvedValue("sessionA")
  c.submit.mockReset().mockResolvedValue(77)
  c.cancel.mockResolvedValue(undefined)
  const [state, set] = createSignal<any>()
  c.status = state
  setStatus = set
  c.defaultPath = () => "/synthetic/A"
  setInstances([])
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  setInstances([])
})
const valid = (name: string, filename = name) => ({
  Valid: { filename, instance_name: name, warnings: [] }
})
function mount() {
  const root = document.createElement("div")
  document.body.append(root)
  const back = vi.fn()
  dispose = render(
    () => (
      <SingleEntity
        entity={{ entity: "MultiMC", selection_type: "directory" } as any}
        setEntity={back}
      />
    ),
    root
  )
  return { root, back }
}
function status(session: string, rows: any[], extra: any = {}) {
  setStatus({
    session,
    scanning: false,
    status: { MultiResult: rows },
    error: undefined,
    ...extra
  })
}
function choose(root: HTMLElement, name: string) {
  const span = Array.from(root.querySelectorAll("span")).find(
    (x) => x.textContent === name
  )!
  span.click()
}
async function changePath(root: HTMLElement, path: string) {
  input(root, 0, path)
  root
    .querySelector("input")!
    .dispatchEvent(new Event("blur", { bubbles: true }))
  await settle()
}
it.each(["MultiResult", "SingleResult"])(
  "a %s row with left-out settings says so and can still be selected",
  async (shape) => {
    const { root } = mount()
    await settle()
    const row = {
      Valid: {
        filename: "pack",
        instance_name: "Packwiz",
        warnings: [{ translation: "InstanceImportCommandsSkipped" }]
      }
    }
    setStatus({
      session: "sessionA",
      scanning: false,
      status:
        shape === "MultiResult"
          ? { MultiResult: [row] }
          : { SingleResult: row },
      error: undefined
    })
    await settle()
    expect(root.textContent).toContain("InstanceImportCommandsSkipped")
    choose(root, "Packwiz")
    expect(instances()).toEqual(["sessionA:0"])
  }
)
it("0109 current mounted rows preserve backend indices after display sort and accepted requests close once", async () => {
  const { root } = mount()
  await settle()
  status("sessionA", [valid("Zulu"), valid("Alpha")])
  choose(root, "Alpha")
  button(root, "_trn_begin_import").click()
  await settle()
  expect(c.submit.mock.calls.map(([x]) => x)).toEqual([
    { index: 1, name: "Alpha", session: "sessionA" }
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
  expect(instances()).toEqual([])
})
it("0109 target path replacement clears selection immediately and rejects old out-of-order status", async () => {
  const { root } = mount()
  await settle()
  status("sessionA", [valid("A only")])
  choose(root, "A only")
  const pending = deferred<string>()
  c.scan.mockReturnValueOnce(pending.promise)
  await changePath(root, "/synthetic/B")
  expect(instances()).toEqual([])
  expect(globalInstances()).toEqual([])
  expect(button(root, "_trn_begin_import").disabled).toBe(true)
  status("sessionA", [valid("Stale A")])
  expect(root.textContent).not.toContain("Stale A")
  pending.resolve("sessionB")
  await settle()
  status("sessionB", [valid("B only")])
  expect(root.textContent).toContain("B only")
  status("sessionA", [valid("Stale A")])
  expect(root.textContent).not.toContain("Stale A")
  expect(c.submit).not.toHaveBeenCalled()
})
it("0109 late target acceptance cannot replace newer accepted session", async () => {
  const first = deferred<string>()
  c.scan.mockReturnValueOnce(first.promise)
  const { root } = mount()
  await settle()
  c.scan.mockResolvedValueOnce("sessionB")
  await changePath(root, "/synthetic/B")
  first.resolve("sessionA")
  await settle()
  status("sessionB", [valid("Current B")])
  expect(root.textContent).toContain("Current B")
  status("sessionA", [valid("Old A")])
  expect(root.textContent).not.toContain("Old A")
})
it("0109 new target NoResults leaves no prior selections or import dispatch", async () => {
  const { root } = mount()
  await settle()
  status("sessionA", [valid("A only")])
  choose(root, "A only")
  c.scan.mockResolvedValueOnce("sessionB")
  await changePath(root, "/synthetic/B")
  setStatus({ session: "sessionB", scanning: false, status: "NoResults" })
  expect(instances()).toEqual([])
  expect(globalInstances()).toEqual([])
  expect(button(root, "_trn_begin_import").disabled).toBe(true)
  button(root, "_trn_begin_import").click()
  expect(c.submit).not.toHaveBeenCalled()
})
it("0109 invalid selected membership is refused locally without negative index or closure", async () => {
  const { root } = mount()
  await settle()
  status("sessionA", [valid("A only")])
  setInstances(["sessionA:99"])
  ;(button(root, "_trn_begin_import") as any).invoke()
  await settle()
  expect(c.submit).not.toHaveBeenCalled()
  expect(c.close).not.toHaveBeenCalled()
  // The translated guidance alone, without the English cause under it.
  const alert = root.querySelector('[role="alert"]')?.textContent
  expect(alert).toContain("_trn_import_batch_uncertain")
  expect(alert).not.toContain("Error:")
  expect(alert).not.toContain("Selected import is unavailable")
})
it("0109 mixed accepted/rejected awaits all rows then preserves uncertain dialog and refuses same-session replay", async () => {
  const rejected = deferred()
  const accepted = deferred()
  c.submit
    .mockReturnValueOnce(rejected.promise)
    .mockReturnValueOnce(accepted.promise)
  const { root } = mount()
  await settle()
  status("sessionA", [valid("Alpha"), valid("Beta")])
  choose(root, "Alpha")
  choose(root, "Beta")
  button(root, "_trn_begin_import").click()
  rejected.reject(new Error("synthetic acceptance uncertain"))
  await settle()
  expect(button(root, "_trn_begin_import").dataset.loading).toBe("true")
  expect(c.close).not.toHaveBeenCalled()
  accepted.resolve(7)
  await settle()
  expect(button(root, "_trn_begin_import").dataset.loading).toBe("false")
  expect(c.close).not.toHaveBeenCalled()
  expect(root.querySelector('[role="alert"]')?.textContent).toContain(
    "_trn_import_batch_uncertain"
  )
  choose(root, "Alpha")
  ;(button(root, "_trn_begin_import") as any).invoke()
  await settle()
  expect(c.submit).toHaveBeenCalledTimes(2)
  const alert = root.querySelector('[role="alert"]')?.textContent
  expect(alert).toContain("_trn_import_batch_uncertain")
  expect(alert).not.toContain("Error:")
  expect(alert).not.toContain("fresh scan")
  expect(alert).not.toContain("synthetic acceptance uncertain")
})
it("0109 stopped partial rows remain usable with error and explicit Retry produces a new session", async () => {
  const { root } = mount()
  await settle()
  status("sessionA", [valid("Partial")], { error: "synthetic scanner stopped" })
  expect(root.querySelector('[role="alert"]')?.textContent).toContain(
    "synthetic scanner stopped"
  )
  choose(root, "Partial")
  button(root, "_trn_begin_import").click()
  await settle()
  expect(c.submit).toHaveBeenCalledWith({
    index: 0,
    name: "Partial",
    session: "sessionA"
  })
  c.scan.mockResolvedValueOnce("sessionRetry")
  button(root, "_trn_import_retry").click()
  await settle()
  status("sessionRetry", [valid("Partial")])
  expect(instances()).toEqual([])
  expect(c.scan).toHaveBeenCalledTimes(2)
})
it("0109 target rejection remains visible and back resets selections", async () => {
  c.scan.mockRejectedValueOnce(new Error("synthetic target refusal"))
  const { root, back } = mount()
  await settle()
  const alert = root.querySelector('[role="alert"]')?.textContent
  expect(alert).toContain("synthetic target refusal")
  expect(alert).not.toContain("Error:")
  expect(button(root, "_trn_begin_import").disabled).toBe(true)
  setInstances(["stale"])
  button(root, "_trn_go_back").click()
  expect(instances()).toEqual([])
  expect(back).toHaveBeenCalledWith(undefined)
})
it("0109 disposed pending batch releases busy without stale close/error continuation", async () => {
  const wait = deferred()
  c.submit.mockReturnValue(wait.promise)
  const { root } = mount()
  await settle()
  status("sessionA", [valid("Alpha")])
  choose(root, "Alpha")
  button(root, "_trn_begin_import").click()
  dispose!()
  dispose = undefined
  wait.resolve(7)
  await settle()
  expect(c.close).not.toHaveBeenCalled()
  expect(c.cancel).toHaveBeenCalled()
})
it.each([false, true])(
  "0108 actual checkbox pointerup then click toggles exactly once from selected=%j",
  (selected) => {
    setInstances(selected ? ["row"] : [])
    const root = document.createElement("div")
    document.body.append(root)
    dispose = render(
      () => (
        <SingleCheckBox title="row" filename="row" setList={setInstances} />
      ),
      root
    )
    const checkbox = root.querySelector("div.rounded-md") as HTMLElement
    pointerGesture(checkbox)
    expect(instances()).toEqual(selected ? [] : ["row"])
  }
)
it("0108 row outside checkbox toggles once and invalid row never selects", () => {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <>
        <SingleCheckBox title="row" filename="row" setList={setInstances} />
        <SingleCheckBox
          title="bad"
          invalidReason="invalid"
          setList={setInstances}
        />
      </>
    ),
    root
  )
  choose(root, "row")
  expect(instances()).toEqual(["row"])
  choose(root, "bad")
  expect(instances()).toEqual(["row"])
})
