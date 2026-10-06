import { afterEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { render } from "solid-js/web"
import {
  createQuery,
  QueryClient,
  QueryClientProvider
} from "@tanstack/solid-query"
import { toast, useSomoto } from "somoto"

vi.mock("@gd/ui", async () => ({
  toast: (await import("somoto")).toast,
  Progress: () => <span />
}))
vi.mock("@gd/i18n", () => ({
  i18n: { language: "english", on: vi.fn(), t: (key: string) => key }
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    createQuery: (opts: any) =>
      createQuery(() => ({
        ...opts(),
        queryFn: async () => ({ releaseChannel: "stable", theme: "dark" })
      }))
  }
}))
const disposers: (() => void)[] = []
let client: QueryClient | undefined
let revision = 0
let listener: ((_: unknown, state: any) => void) | undefined
let updaterModule: typeof import("./updater") | undefined
const flush = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve()
}
afterEach(() => {
  disposers.splice(0).forEach((d) => d())
  client?.clear()
  client = undefined
  toast.dismiss()
  vi.useRealTimers()
  vi.unstubAllGlobals()
  document.body.textContent = ""
})
async function fixture() {
  vi.stubGlobal("__APP_VERSION__", "0.0.0-test")
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: true }),
    getUpdateState: async () => null,
    onUpdateStateChanged: (cb: any) => {
      listener = cb
    },
    installUpdate: vi.fn(),
    checkForUpdates: vi.fn()
  })
  updaterModule ??= await import("./updater")
  const updater = updaterModule
  let store!: ReturnType<typeof useSomoto>
  createRoot((d) => {
    disposers.push(d)
    store = useSomoto()
  })
  await flush()
  let readyUpdate: { id: number; version: string } | null = null
  const emit = async (state: string, extra = {}) => {
    if (state === "downloaded")
      readyUpdate = { id: revision + 1, version: "next" }
    else if (!["idle", "no-update", "checking"].includes(state))
      readyUpdate = null
    listener!(null, {
      revision: ++revision,
      state,
      progress: 30,
      readyUpdate,
      updateInfo: { version: "next", downloadUrl: null },
      ...extra
    })
    await flush()
  }
  await emit("error")
  return { updater, store, emit }
}
it("Task23 carrier removes failed progress in actual Somoto active store and retry restores fresh progress", async () => {
  const f = await fixture()
  await f.emit("downloading")
  expect(
    f.store.toasts().find((t) => t.id === "updater-downloading")
  ).toMatchObject({ type: "loading", duration: Infinity, closeButton: false })
  await f.emit("error", { error: { message: "synthetic download failed" } })
  expect(f.store.toasts().some((t) => t.id === "updater-downloading")).toBe(
    false
  )
  expect(
    f.store.toasts().find((t) => t.id === "updater-checking")
  ).toMatchObject({ type: "error", duration: 8000 })
  await f.emit("downloading")
  expect(
    f.store.toasts().find((t) => t.id === "updater-downloading")?.type
  ).toBe("loading")
})
it("ready success restores merged close button after unclosable progress", async () => {
  const f = await fixture()
  await f.emit("downloading")
  await f.emit("downloaded")
  expect(
    f.store.toasts().find((t) => t.id === "updater-downloading")
  ).toMatchObject({ type: "success", closeButton: true })
})
it.each(["idle", "no-update"])(
  "%s preserves genuine completed pending prompt and Install action",
  async (state) => {
    const f = await fixture()
    await f.emit("downloading")
    await f.emit("downloaded")
    await f.emit(state)
    expect(f.updater.hasPendingUpdate()).toBe(true)
    const ready = f.store.toasts().find((t) => t.id === "updater-downloading")!
    expect(ready.type).toBe("success")
    expect(ready.action).toBeDefined()
    expect(ready.cancel).toBeDefined()
  }
)
it("pre-download error has no progress ghost and stale revisions cannot remove ready state", async () => {
  const f = await fixture()
  await f.emit("error")
  expect(f.store.toasts().some((t) => t.id === "updater-downloading")).toBe(
    false
  )
  await f.emit("downloaded")
  listener!(null, { state: "error", revision: revision - 1 })
  await flush()
  expect(f.updater.hasPendingUpdate()).toBe(true)
})
it("same-channel settings refresh preserves one periodic timer; genuine channel change and remount replace it", async () => {
  vi.useFakeTimers()
  const f = await fixture()
  client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: Infinity, staleTime: Infinity }
    }
  })
  client.setQueryData(["settings.getSettings"], {
    releaseChannel: "stable",
    theme: "dark"
  })
  const mount = () => {
    const root = document.createElement("div")
    document.body.append(root)
    const Content = () => {
      void f.updater.checkForUpdates()
      return <span />
    }
    const d = render(
      () => (
        <QueryClientProvider client={client!}>
          <Content />
        </QueryClientProvider>
      ),
      root
    )
    disposers.push(d)
    return d
  }
  const d = mount()
  await flush()
  expect(window.checkForUpdates).toHaveBeenLastCalledWith("stable")
  await vi.advanceTimersByTimeAsync(30 * 60 * 1000)
  expect(window.checkForUpdates).toHaveBeenCalledTimes(2)
  client.setQueryData(["settings.getSettings"], {
    releaseChannel: "stable",
    theme: "light"
  })
  await flush()
  await vi.advanceTimersByTimeAsync(0)
  await vi.advanceTimersByTimeAsync(30 * 60 * 1000)
  expect(window.checkForUpdates).toHaveBeenCalledTimes(3)
  client.setQueryData(["settings.getSettings"], {
    releaseChannel: "beta",
    theme: "light"
  })
  await flush()
  await vi.advanceTimersByTimeAsync(0)
  expect(window.checkForUpdates).toHaveBeenLastCalledWith("beta")
  await vi.advanceTimersByTimeAsync(30 * 60 * 1000)
  expect(window.checkForUpdates).toHaveBeenCalledTimes(5)
  d()
  const before = vi.mocked(window.checkForUpdates).mock.calls.length
  await vi.advanceTimersByTimeAsync(30 * 60 * 1000)
  expect(window.checkForUpdates).toHaveBeenCalledTimes(before)
  mount()
  await flush()
  expect(window.checkForUpdates).toHaveBeenCalledTimes(before + 1)
})
