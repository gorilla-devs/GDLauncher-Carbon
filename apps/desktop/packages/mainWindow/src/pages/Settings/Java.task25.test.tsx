import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"

const control = vi.hoisted(() => ({
  params: null as any,
  state: null as any,
  mutation: vi.fn(),
  options: null as any,
  cache: new Map<string, any>(),
  cancel: vi.fn(),
  invalidate: vi.fn()
}))
vi.mock("@gd/ui", () => import("@/test/task25/ui"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({ useParams: () => control.params }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/components/SearchInputContext", () => ({
  default: () => ({ setSelectedInstanceId: vi.fn() })
}))
vi.mock("@/components/JavaPathAutoComplete", () => ({ default: () => <div /> }))
vi.mock("./settings.java.data", () => ({
  default: () => ({
    availableJavas: { data: {} },
    javaProfiles: { data: [] },
    totalRam: { data: 8589934592 }
  })
}))
vi.mock("@/utils/rspcClient", () => ({
  apiUrl: (path: string) => path,
  queryClient: {
    cancelQueries: (options: any) => control.cancel(options),
    getQueryData: (key: any) => control.cache.get(JSON.stringify(key)),
    setQueryData: (key: any, value: any) => {
      const id = JSON.stringify(key)
      control.cache.set(
        id,
        typeof value === "function" ? value(control.cache.get(id)) : value
      )
    },
    invalidateQueries: (options: any) => control.invalidate(options)
  },
  rspc: {
    createQuery: (options: any) => ({
      get data() {
        const key = options().queryKey[0]
        if (key === "instance.getInstanceDetails") return control.state.details
        if (key === "settings.getSettings") return control.state.settings
        if (key === "systeminfo.getTotalRAM") return 8589934592
        return []
      }
    }),
    createMutation: (options: any) => {
      if (options().mutationKey[0] === "instance.updateInstance")
        control.options = options()
      return {
        mutate: (payload: any) =>
          control.mutation(options().mutationKey[0], payload)
      }
    }
  }
}))
import Java from "./Java"
import Settings from "../Library/Instance/Tabs/Settings"

let root: HTMLDivElement
let dispose: () => void
let setState: any
let setParams: any
const details = (id: number, args: string | null) => ({
  id,
  name: `instance-${id}`,
  extraJavaArgs: args,
  globalJavaArgs: true,
  memory: null,
  javaOverride: null,
  gameResolution: null,
  modpack: null
})
beforeEach(() => {
  vi.clearAllMocks()
  control.cache.clear()
  const [state, setter] = createStore({
    details: details(1, ""),
    settings: { javaCustomArgs: "", xmx: 2048 }
  })
  const [params, paramSetter] = createStore({ id: "1" })
  control.state = state
  control.params = params
  setState = setter
  setParams = paramSetter
  control.cancel.mockResolvedValue(undefined)
})
afterEach(() => {
  dispose?.()
  root?.remove()
})
function mount(global = false) {
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => (global ? <Java /> : <Settings />), root)
}
function reset() {
  return root
    .querySelector(".i-hugeicons\\:arrow-turn-backward")!
    .closest("button")!
}

test("global loaded empty Java baseline survives an edit and actual Reset writes empty", () => {
  mount(true)
  setState("settings", "javaCustomArgs", "-Dsynthetic=edited")
  reset().click()
  expect(control.mutation).toHaveBeenLastCalledWith("settings.setSettings", {
    javaCustomArgs: { Set: "" }
  })
})
test.each(["", null, "-Dsynthetic=original"])(
  "instance loaded %s baseline survives editing and actual Reset restores it",
  (baseline) => {
    setState("details", details(1, baseline))
    mount()
    setState("details", "extraJavaArgs", "-Dsynthetic=edited")
    reset().click()
    expect(control.mutation).toHaveBeenLastCalledWith(
      "instance.updateInstance",
      { instance: 1, extraJavaArgs: { Set: baseline } }
    )
  }
)
test("route reuse rejects retained A data while B loads, then Reset restores B's loaded baseline", () => {
  setState("details", details(1, "-Dsynthetic=A"))
  mount()
  setParams("id", "2")
  reset().click()
  expect(control.mutation).not.toHaveBeenCalled()
  setState("details", details(2, "-Dsynthetic=B"))
  setState("details", "extraJavaArgs", "-Dsynthetic=editB")
  reset().click()
  expect(control.mutation).toHaveBeenLastCalledWith("instance.updateInstance", {
    instance: 2,
    extraJavaArgs: { Set: "-Dsynthetic=B" }
  })
})
test("unloaded global data is distinct from an empty baseline", () => {
  setState("settings", undefined)
  mount(true)
  reset().click()
  expect(control.mutation).not.toHaveBeenCalled()
  setState("settings", { javaCustomArgs: "-Dsynthetic=loaded", xmx: 2048 })
  setState("settings", "javaCustomArgs", "-Dsynthetic=edited")
  reset().click()
  expect(control.mutation).toHaveBeenLastCalledWith("settings.setSettings", {
    javaCustomArgs: { Set: "-Dsynthetic=loaded" }
  })
})
test("late mutation optimism, rollback and invalidation all use captured variables after route change", async () => {
  mount()
  const keyA = ["instance.getInstanceDetails", 1]
  const keyB = ["instance.getInstanceDetails", 2]
  control.cache.set(JSON.stringify(keyA), details(1, "-Dsynthetic=A"))
  control.cache.set(JSON.stringify(keyB), details(2, "-Dsynthetic=B"))
  let release!: () => void
  control.cancel.mockReturnValueOnce(
    new Promise<void>((resolve) => {
      release = resolve
    })
  )
  const variables = { instance: 1, extraJavaArgs: { Set: "-Dsynthetic=editA" } }
  const pending = control.options.onMutate(variables)
  setParams("id", "2")
  release()
  const context = await pending
  expect(control.cache.get(JSON.stringify(keyA)).extraJavaArgs).toBe(
    "-Dsynthetic=editA"
  )
  expect(control.cache.get(JSON.stringify(keyB)).extraJavaArgs).toBe(
    "-Dsynthetic=B"
  )
  control.options.onError(new Error("Synthetic rejection"), variables, context)
  expect(control.cache.get(JSON.stringify(keyA)).extraJavaArgs).toBe(
    "-Dsynthetic=A"
  )
  expect(control.cache.get(JSON.stringify(keyB)).extraJavaArgs).toBe(
    "-Dsynthetic=B"
  )
  control.options.onSettled(undefined, null, variables, context)
  expect(control.invalidate).toHaveBeenCalledWith({ queryKey: keyA })
})
test("clear remains empty and prepend-global toggling remains independent of Reset", () => {
  setState("details", details(1, "-Dsynthetic=A"))
  mount()
  const clear = [...root.querySelectorAll("button")].find((button) =>
    button.querySelector(".i-hugeicons\\:cancel-01")
  )!
  clear.click()
  expect(control.mutation).toHaveBeenLastCalledWith("instance.updateInstance", {
    instance: 1,
    extraJavaArgs: { Set: "" }
  })
  expect(root.textContent).toContain("{GLOBAL_JAVA_ARGS}")
})
