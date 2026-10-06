import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { Show, createSignal } from "solid-js"
import { render } from "solid-js/web"
import { useModInstallation } from "@/components/ModDownloadButton/hooks/useModInstallation"
import ShaderLoaderSetup from "."
import { shaderInstallRunning } from "./state"
import { button, deferred, settle, toast } from "@/task33.fixture"
const c = vi.hoisted(() => ({
  query: vi.fn(),
  mutation: vi.fn(),
  close: vi.fn(),
  complete: vi.fn(),
  open: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", async () => {
  const { default: notifications } =
    await import("../../../../../../../../../packages/i18n/locale/english/notifications.json")
  const translate = (k: string) =>
    k.startsWith("notifications:")
      ? (notifications[
          k.slice("notifications:".length) as keyof typeof notifications
        ] ?? k)
      : k
  return {
    Trans: (p: any) => <span>{p.key}</span>,
    useTransContext: () => [translate],
    i18n: { t: translate }
  }
})
vi.mock("@gd/i18n/helpers", () => ({ getTaskTranslationKey: (k: string) => k }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close, openModal: c.open })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/rspcClient", async () => ({
  ...(await vi.importActual<any>("@/utils/rspcClient")),
  rspc: {
    useContext: () => ({ client: { query: c.query, mutation: c.mutation } }),
    createMutation: (f: any) => ({
      mutateAsync: (v: any) => c.mutation([f().mutationKey, v])
    })
  }
}))
let dispose: undefined | (() => void)
const verification = {
  instance_id: 7,
  mc_version: "1.20.1",
  modloaders: ["fabric:0.15.11"],
  filename: "iris.jar",
  sha1: "synthetic",
  size: 2
}
const failed = {
  type: "Failed",
  value: {
    progress: {
      type: "Failed",
      value: { cause: [{ display: "synthetic stage failed" }] }
    },
    active_subtasks: []
  }
}
const rpcError = new Error(
  JSON.stringify({
    code: 500,
    message: JSON.stringify({
      cause: [{ display: "Readable shader request failed" }],
      backtrace: "synthetic trace withheld"
    })
  })
)
beforeEach(() => {
  vi.clearAllMocks()
  c.close.mockReset()
  c.open.mockReset()
  c.query
    .mockReset()
    .mockImplementation(async ([key]: any) =>
      key === "instance.verifyShaderLoader"
        ? true
        : key === "instance.checkShaderRequirements"
          ? { kind: "LoaderPresent" }
          : { type: "Succeeded" }
    )
  c.mutation
    .mockReset()
    .mockImplementation(async ([key]: any) =>
      key === "instance.installFabricLoaderDefault"
        ? 11
        : key === "instance.ensureShaderLoader"
          ? { task_id: 12, verification }
          : key === "vtask.dismissTask"
            ? undefined
            : 13
    )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.useRealTimers()
})
async function mount(fileOnly = false) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <ShaderLoaderSetup
        title="Synthetic"
        data={{
          instanceId: 7,
          recommendation: {
            kind: "RequiresModloader",
            mc_version: "1.20.1",
            recommended: "Iris"
          },
          installLatest: false,
          modSource: {
            Modrinth: { project_id: "shaderA", version_id: "verA" }
          },
          onComplete: c.complete
        }}
      />
    ),
    root
  )
  button(root, fileOnly ? "continue_anyway" : "auto_setup").click()
  await settle()
  await settle()
  return root
}
const calls = (key: string) =>
  c.mutation.mock.calls.filter((x) => x[0][0] === key)
it("M20 serialized RPC rejection renders readable message without payload/backtrace", async () => {
  c.mutation.mockImplementation(async ([key]: any) => {
    if (key === "instance.installFabricLoaderDefault") return 11
    if (key === "instance.ensureShaderLoader")
      return { task_id: 12, verification }
    throw rpcError
  })
  const root = await mount()
  expect(root.textContent).toContain("Readable shader request failed")
  expect(root.textContent).not.toContain('"code"')
  expect(root.textContent).not.toContain("synthetic trace withheld")
  expect(shaderInstallRunning()).toBe(false)
})
it("M20 Retry retains positively settled Fabric/loader stages and resumes only accepted failed shader", async () => {
  c.query.mockImplementation(async ([key, id]: any) =>
    key === "instance.verifyShaderLoader"
      ? true
      : key === "vtask.getTaskOutcome" && id === 13
        ? failed
        : { type: "Succeeded" }
  )
  let shader = 0
  c.mutation.mockImplementation(async ([key]: any) =>
    key === "instance.installFabricLoaderDefault"
      ? 11
      : key === "instance.ensureShaderLoader"
        ? { task_id: 12, verification }
        : key === "vtask.dismissTask"
          ? undefined
          : ++shader === 1
            ? 13
            : 14
  )
  const root = await mount()
  expect(root.textContent).toContain("synthetic stage failed")
  button(root, "retry").click()
  await settle()
  await settle()
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
  expect(calls("instance.ensureShaderLoader")).toHaveLength(1)
  expect(calls("vtask.dismissTask")[0][0]).toEqual(["vtask.dismissTask", 13])
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(14)
})
it("M20 failed Fabric retries only guarded same-session preparation and waits for positive replacement", async () => {
  const replacement = deferred()
  c.query.mockImplementation(async ([key, id]: any) =>
    key === "instance.verifyShaderLoader"
      ? true
      : key === "vtask.getTaskOutcome" && id === 11
        ? failed
        : key === "vtask.getTaskOutcome" && id === 21
          ? replacement.promise
          : { type: "Succeeded" }
  )
  c.mutation.mockImplementation(async ([key]: any) =>
    key === "instance.installFabricLoaderDefault"
      ? 11
      : key === "instance.prepareInstance"
        ? 21
        : key === "instance.ensureShaderLoader"
          ? { task_id: 12, verification }
          : key === "vtask.dismissTask"
            ? undefined
            : 13
  )
  const root = await mount()
  button(root, "retry").click()
  await settle()
  await settle()
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
  expect(calls("instance.prepareInstance")[0]?.[0]).toEqual([
    "instance.prepareInstance",
    { instance: 7, expected_failed_task: 11, expected_mc_version: "1.20.1" }
  ])
  expect(calls("instance.ensureShaderLoader")).toHaveLength(0)
  replacement.resolve({ type: "Succeeded" })
  await settle()
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(13)
})
it("M20 failed task acknowledgement rejection starts no replacement and stays usable", async () => {
  c.query.mockResolvedValue(failed)
  c.mutation.mockImplementation(async ([key]: any) => {
    if (key === "vtask.dismissTask")
      throw new Error("synthetic acknowledgement denied")
    return 11
  })
  const root = await mount()
  button(root, "retry").click()
  await settle()
  await settle()
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
  expect(root.textContent).toContain("synthetic acknowledgement denied")
  expect(shaderInstallRunning()).toBe(false)
  expect(c.complete).not.toHaveBeenCalled()
  button(root, "shader_loader_cancel").click()
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(null)
  expect(toast).toHaveBeenLastCalledWith(
    "Shader install cancelled — steps that already finished were kept",
    { duration: 4000 }
  )
})
it("M20 positive ordinary stages and fileOnly preserve exact addon target and complete once", async () => {
  await mount()
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(13)
  expect(calls("instance.installMod")[0][0][1]).toEqual({
    instance_id: 7,
    mod_source: { Modrinth: { project_id: "shaderA", version_id: "verA" } },
    install_deps: true,
    replaces_mod: null
  })
  dispose?.()
  dispose = undefined
  vi.clearAllMocks()
  const second = await mount(true)
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(0)
  expect(calls("instance.ensureShaderLoader")).toHaveLength(0)
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(13)
  expect(second.textContent).not.toContain("stage failed")
})
it("M20 disposed accepted task retires continuation and callback remains once", async () => {
  const p = deferred()
  c.query.mockReturnValue(p.promise)
  await mount()
  dispose?.()
  dispose = undefined
  p.resolve({ type: "Succeeded" })
  await settle()
  expect(calls("instance.ensureShaderLoader")).toHaveLength(0)
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(null)
  expect(toast).toHaveBeenLastCalledWith(
    "Shader install cancelled — steps that already finished were kept",
    { duration: 4000 }
  )
  expect(shaderInstallRunning()).toBe(false)
})
it("M20 no-ID rejection offers Cancel and fresh actual caller preflight, never ambiguous retry", async () => {
  const [modal, setModal] = createSignal<any>()
  let preflight = 0
  c.query.mockImplementation(async ([key]: any) =>
    key === "instance.checkShaderRequirements"
      ? {
          kind: "RequiresModloader",
          mc_version: ++preflight === 1 ? "1.20.1" : "1.21",
          loader_modrinth_id: "iris"
        }
      : key === "instance.verifyShaderLoader"
        ? true
        : { type: "Succeeded" }
  )
  c.mutation.mockImplementation(async ([key]: any) => {
    if (key === "instance.installFabricLoaderDefault") return 11
    if (key === "instance.ensureShaderLoader")
      return { task_id: 12, verification }
    throw rpcError
  })
  c.open.mockImplementation((_name: any, data: any) => setModal(data))
  c.close.mockImplementation(() => setModal(undefined))
  let caller: any
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => {
    caller = useModInstallation({
      addon: { type: "shader", platform: "modrinth", id: "shaderA" } as any,
      fileId: "verA"
    })
    return (
      <>
        <button onClick={() => caller.handleInstanceSelection(7)}>
          Choose instance
        </button>
        <Show when={modal()}>
          {(data) => <ShaderLoaderSetup title="Synthetic" data={data()} />}
        </Show>
      </>
    )
  }, root)
  button(root, "Choose instance").click()
  await settle()
  button(root, "auto_setup").click()
  await settle()
  await settle()
  expect(root.textContent).toContain("shader_loader_uncertain")
  expect(
    Array.from(root.querySelectorAll("button")).some((b) =>
      b.textContent?.includes("general:_trn_retry")
    )
  ).toBe(false)
  expect(calls("instance.installMod")).toHaveLength(1)
  button(root, "shader_loader_cancel").click()
  await settle()
  expect(shaderInstallRunning()).toBe(false)
  expect(caller.instanceLoadingStates().has(7)).toBe(false)
  expect(modal()).toBeUndefined()
  expect(toast).toHaveBeenLastCalledWith(
    "Setup closed. Installation may still finish. Check the instance before starting again.",
    { duration: 4000 }
  )
  button(root, "Choose instance").click()
  await settle()
  expect(preflight).toBe(2)
  expect(modal().recommendation.mc_version).toBe("1.21")
  expect(calls("instance.installMod")).toHaveLength(1)
  button(root, "shader_loader_cancel").click()
  await settle()
  expect(toast).toHaveBeenLastCalledWith(
    "Shader install cancelled — no compatible shader loader is installed",
    { duration: 4000 }
  )
})
it("M20 disposed no-ID uncertainty uses the same truthful once-only exit feedback", async () => {
  c.mutation.mockImplementation(async ([key]: any) => {
    if (key === "instance.installFabricLoaderDefault") return 11
    if (key === "instance.ensureShaderLoader")
      return { task_id: 12, verification }
    throw rpcError
  })
  await mount()
  expect(toast).not.toHaveBeenCalled()
  dispose?.()
  dispose = undefined
  expect(toast).toHaveBeenCalledExactlyOnceWith(
    "Setup closed. Installation may still finish. Check the instance before starting again.",
    { duration: 4000 }
  )
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(null)
  expect(shaderInstallRunning()).toBe(false)
  await settle()
  expect(toast).toHaveBeenCalledTimes(1)
  expect(calls("instance.installMod")).toHaveLength(1)
})
it("M20 accepted read errors and Known100 progress cannot checkpoint before Succeeded", async () => {
  vi.useFakeTimers()
  let reads = 0
  c.query.mockImplementation(async () => {
    if (++reads === 1) throw new Error("synthetic read failure")
    return reads === 2
      ? {
          type: "Running",
          value: { progress: { type: "Known", value: 1 }, active_subtasks: [] }
        }
      : { type: "Succeeded" }
  })
  await mount(true)
  expect(c.complete).not.toHaveBeenCalled()
  expect(shaderInstallRunning()).toBe(true)
  await vi.advanceTimersByTimeAsync(200)
  expect(c.complete).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(200)
  await settle()
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(13)
})
it("M20 pending acknowledgement direct repeat is inert; disposal retires late continuation", async () => {
  const ack = deferred()
  c.query.mockResolvedValue(failed)
  c.mutation.mockImplementation(async ([key]: any) =>
    key === "vtask.dismissTask" ? ack.promise : 11
  )
  const root = await mount()
  const retry = button(root, "retry")
  retry.click()
  void (retry as any).invoke()
  await settle()
  expect(calls("vtask.dismissTask")).toHaveLength(1)
  dispose?.()
  dispose = undefined
  ack.resolve(undefined)
  await settle()
  expect(calls("instance.prepareInstance")).toHaveLength(0)
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
})
it("M20 first-observed Dismissed Fabric uses guarded recovery and waits for replacement success", async () => {
  const replacement = deferred()
  c.query.mockImplementation(async ([key, id]: any) =>
    key === "instance.verifyShaderLoader"
      ? true
      : key === "vtask.getTaskOutcome" && id === 11
        ? { type: "Dismissed" }
        : key === "vtask.getTaskOutcome" && id === 21
          ? replacement.promise
          : { type: "Succeeded" }
  )
  c.mutation.mockImplementation(async ([key]: any) =>
    key === "instance.installFabricLoaderDefault"
      ? 11
      : key === "instance.prepareInstance"
        ? 21
        : key === "instance.ensureShaderLoader"
          ? { task_id: 12, verification }
          : 13
  )
  const root = await mount()
  expect(root.textContent).toContain("This failed attempt was dismissed")
  const retry = button(root, "retry")
  retry.click()
  await settle()
  await settle()
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
  expect(calls("instance.prepareInstance").map((call) => call[0])).toEqual([
    [
      "instance.prepareInstance",
      { instance: 7, expected_failed_task: 11, expected_mc_version: "1.20.1" }
    ]
  ])
  expect(calls("vtask.dismissTask")).toHaveLength(0)
  expect(calls("instance.ensureShaderLoader")).toHaveLength(0)
  expect(c.complete).not.toHaveBeenCalled()
  expect(shaderInstallRunning()).toBe(true)
  replacement.resolve({ type: "Succeeded" })
  await settle()
  await settle()
  expect(calls("instance.ensureShaderLoader")).toHaveLength(1)
  expect(c.complete).toHaveBeenCalledExactlyOnceWith(13)
  expect(c.close).toHaveBeenCalledTimes(1)
  void (retry as any).invoke()
  dispose?.()
  dispose = undefined
  await settle()
  expect(calls("instance.prepareInstance")).toHaveLength(1)
  expect(c.complete).toHaveBeenCalledTimes(1)
})
it.each(["loader", "shader"])(
  "M20 first-observed Dismissed %s preserves positive prior stages and exact replacement ownership",
  async (stage) => {
    const replacement = deferred()
    const retiredId = stage === "loader" ? 12 : 13
    const replacementId = stage === "loader" ? 22 : 23
    let loaderRuns = 0,
      shaderRuns = 0
    c.query.mockImplementation(async ([key, id]: any) =>
      key === "instance.verifyShaderLoader"
        ? true
        : key === "vtask.getTaskOutcome" && id === retiredId
          ? { type: "Dismissed" }
          : key === "vtask.getTaskOutcome" && id === replacementId
            ? replacement.promise
            : { type: "Succeeded" }
    )
    c.mutation.mockImplementation(async ([key]: any) => {
      if (key === "instance.installFabricLoaderDefault") return 11
      if (key === "instance.ensureShaderLoader")
        return {
          task_id: ++loaderRuns === 1 ? 12 : 22,
          verification
        }
      if (key === "instance.installMod") return ++shaderRuns === 1 ? 13 : 23
      throw new Error("Unexpected recovery mutation")
    })
    const root = await mount()
    expect(root.textContent).toContain("This failed attempt was dismissed")
    const retry = button(root, "retry")
    retry.click()
    await settle()
    await settle()
    expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
    expect(calls("instance.prepareInstance")).toHaveLength(0)
    expect(calls("vtask.dismissTask")).toHaveLength(0)
    expect(calls("instance.ensureShaderLoader")).toHaveLength(
      stage === "loader" ? 2 : 1
    )
    expect(calls("instance.installMod")).toHaveLength(
      stage === "shader" ? 2 : 0
    )
    expect(
      c.query.mock.calls.some(
        (call) =>
          call[0][0] === "instance.verifyShaderLoader" &&
          call[0][1] === verification
      )
    ).toBe(true)
    expect(c.complete).not.toHaveBeenCalled()
    replacement.resolve({ type: "Succeeded" })
    await settle()
    await settle()
    expect(c.complete).toHaveBeenCalledExactlyOnceWith(
      stage === "loader" ? 13 : 23
    )
    expect(c.close).toHaveBeenCalledTimes(1)
    const shaderInput = calls("instance.installMod").at(-1)?.[0][1]
    expect(shaderInput).toEqual({
      instance_id: 7,
      mod_source: { Modrinth: { project_id: "shaderA", version_id: "verA" } },
      install_deps: true,
      replaces_mod: null
    })
    void (retry as any).invoke()
    dispose?.()
    dispose = undefined
    await settle()
    expect(c.complete).toHaveBeenCalledTimes(1)
    expect(calls("instance.installMod")).toHaveLength(
      stage === "shader" ? 2 : 1
    )
  }
)
it("M20 Unavailable is not acknowledged failure and Retry only reobserves the accepted task", async () => {
  c.query.mockResolvedValue({ type: "Unavailable" })
  const root = await mount()
  expect(root.textContent).toContain(
    "Couldn't confirm whether this addon change finished"
  )
  button(root, "retry").click()
  await settle()
  await settle()
  expect(calls("instance.installFabricLoaderDefault")).toHaveLength(1)
  expect(calls("instance.prepareInstance")).toHaveLength(0)
  expect(calls("instance.ensureShaderLoader")).toHaveLength(0)
  expect(calls("vtask.dismissTask")).toHaveLength(0)
  expect(c.complete).not.toHaveBeenCalled()
  expect(
    c.query.mock.calls
      .filter((call) => call[0][0] === "vtask.getTaskOutcome")
      .map((call) => call[0][1])
  ).toEqual([11, 11])
})
