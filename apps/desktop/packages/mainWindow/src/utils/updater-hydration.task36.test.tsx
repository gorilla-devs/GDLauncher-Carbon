import { expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { toast, useSomoto } from "somoto"
import { createUpdaterController } from "../../../main/utils/updaterController"
vi.mock("@gd/ui", async () => ({
  toast: (await import("somoto")).toast,
  Progress: () => <span />
}))
vi.mock("@gd/i18n", () => ({
  i18n: { language: "english", on: vi.fn(), t: (key: string) => key }
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    createQuery: () => {
      throw Error("No query/transport admitted")
    }
  }
}))
it("fresh no-update hydration exposes genuine downloaded authority once and same-token events preserve dismissal", async () => {
  const events: Record<string, (...args: any[]) => void> = {},
    install = vi.fn(),
    shutdown = vi.fn(async () => {})
  let listener: ((_: unknown, state: any) => void) | undefined
  const controller = createUpdaterController({
    version: "1.0.0",
    artifact: () => null,
    isDownloaded: () => true,
    manual: async () => ({ noUpdate: true }),
    prepareShutdown: shutdown,
    broadcast: (state) => listener?.(null, state),
    updater: {
      channel: null,
      allowPrerelease: false,
      allowDowngrade: false,
      on: (event, cb) => {
        events[event] = cb
      },
      checkForUpdates: async () => {
        events["update-not-available"]()
        return {}
      },
      quitAndInstall: install
    }
  })
  events["update-available"]({
    version: "2.0.0",
    files: [{ url: "fixture.zip", sha512: "synthetic" }]
  })
  events["update-downloaded"]()
  await controller.check("stable")
  expect(controller.getState().state).toBe("no-update")
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: true }),
    getUpdateState: async () => controller.getState(),
    onUpdateStateChanged: (cb: any) => {
      listener = cb
    },
    installUpdate: (id: number) => controller.install(id)
  })
  let store!: ReturnType<typeof useSomoto>, dispose!: () => void
  createRoot((d) => {
    dispose = d
    store = useSomoto()
  })
  try {
    const renderer = await import("./updater")
    for (let i = 0; i < 30; i++) await Promise.resolve()
    const ready = store.toasts().find((t) => t.id === "updater-downloading")!
    expect(ready).toMatchObject({ type: "success", closeButton: true })
    expect(renderer.pendingUpdateVersion()).toBe("2.0.0")
    toast.dismiss("updater-downloading")
    await controller.check("stable")
    for (let i = 0; i < 30; i++) await Promise.resolve()
    expect(store.toasts().some((t) => t.id === "updater-downloading")).toBe(
      false
    )
    renderer.showPendingUpdateToast()
    await (
      store.toasts().find((t) => t.id === "updater-downloading")!.action as any
    ).onClick()
    expect(shutdown).toHaveBeenCalledTimes(1)
    expect(install).toHaveBeenCalledTimes(1)
  } finally {
    dispose()
    toast.dismiss()
  }
})
