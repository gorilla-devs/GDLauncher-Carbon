import { afterEach, expect, it, vi } from "vitest"
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
      throw Error("No transport/query admitted")
    }
  }
}))
let renderer: typeof import("./updater") | undefined,
  listener: ((_: unknown, state: any) => void) | undefined
let dispose: (() => void) | undefined,
  offset = 0
const flush = async () => {
  for (let i = 0; i < 40; i++) await Promise.resolve()
}
afterEach(() => {
  dispose?.()
  toast.dismiss()
  offset += 1000
  vi.unstubAllGlobals()
})
async function fixture(
  shutdown = vi.fn(async () => {}),
  installBlocker?: () => Promise<any>
) {
  const events: Record<string, (...args: any[]) => void> = {},
    install = vi.fn(),
    checks = vi.fn(async () => ({}))
  let failure: unknown,
    eligible = true
  const controller = createUpdaterController({
    version: "1.0.0",
    artifact: () => null,
    manual: async () => ({ noUpdate: true }),
    prepareShutdown: shutdown,
    installBlocker,
    isDownloaded: () => eligible,
    broadcast: (state: any) =>
      listener?.(null, { ...state, revision: state.revision + offset }),
    updater: {
      channel: null,
      allowPrerelease: false,
      allowDowngrade: false,
      on: (event: string, cb: any) => {
        events[event] = cb
      },
      checkForUpdates: checks,
      quitAndInstall: install
    }
  } as any)
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: true }),
    getUpdateState: async () => controller.getState(),
    onUpdateStateChanged: (cb: any) => {
      listener = cb
    },
    checkForUpdates: (channel: any) => controller.check(channel),
    installUpdate: (...args: any[]) => {
      const promise = (controller.install as any)(...args)
      promise.catch((error: unknown) => {
        failure = error
      })
      return promise
    }
  })
  renderer ??= await import("./updater")
  let store!: ReturnType<typeof useSomoto>
  createRoot((d) => {
    dispose = d
    store = useSomoto()
  })
  await flush()
  const download = async (version = "2.0.0") => {
    events["update-available"]({
      version,
      files: [{ url: "fixture.zip", sha512: version }],
      releaseDate: "2026-10-01"
    })
    events["update-downloaded"]()
    await flush()
  }
  await download()
  const shown = (id: string) => store.toasts().find((t) => t.id === id) as any
  const ready = () => shown("updater-downloading")!
  return {
    controller,
    shown,
    events,
    install,
    shutdown,
    checks,
    ready,
    download,
    get failure() {
      return failure
    },
    set eligible(value: boolean) {
      eligible = value
    }
  }
}
it.each(["checking", "no-update"])(
  "genuine downloaded prompt remains installable through later %s producer state",
  async (phase) => {
    const f = await fixture()
    await f.controller.check("stable")
    if (phase === "no-update") f.events["update-not-available"]()
    await flush()
    expect(f.controller.getState().state).toBe(phase)
    await f.ready().action!.onClick()
    await flush()
    expect(f.failure).toBeUndefined()
    expect(f.shutdown).toHaveBeenCalledTimes(1)
    expect(f.install).toHaveBeenCalledTimes(1)
  }
)
it("owned shutdown rejection produces finite closable same-artifact retry through the real toast action", async () => {
  let failed = true
  const f = await fixture(
    vi.fn(async () => {
      if (failed) throw Error("Owned runtime busy; retry")
    })
  )
  await f.ready().action!.onClick()
  await flush()
  expect(f.ready()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  expect(f.ready().action).toBeDefined()
  expect(f.install).not.toHaveBeenCalled()
  failed = false
  await f.ready().action!.onClick()
  await flush()
  expect(f.install).toHaveBeenCalledTimes(1)
})
it("retained old Install action cannot authorize a replacement downloaded artifact", async () => {
  const f = await fixture(),
    oldAction = f.ready().action!
  await f.download("3.0.0")
  await oldAction.onClick()
  await flush()
  expect(f.install).not.toHaveBeenCalled()
  expect(f.ready().type).toBe("success")
  await f.ready().action!.onClick()
  await flush()
  expect(f.install).toHaveBeenCalledTimes(1)
})
it("lost downloaded eligibility refuses before shutdown and recovers the installing toast", async () => {
  const f = await fixture()
  f.eligible = false
  await f.ready().action!.onClick()
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  expect(f.install).not.toHaveBeenCalled()
  expect(f.ready()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
})
it.each(["synchronous", "later"])(
  "%s installer error event with void return recovers the real installing toast",
  async (timing) => {
    const f = await fixture()
    if (timing === "synchronous")
      f.install.mockImplementation(() =>
        f.events.error(Error("Native adapter rejected"))
      )
    await f.ready().action!.onClick()
    await flush()
    if (timing === "later") {
      f.events.error(Error("Native adapter rejected"))
      await flush()
    }
    expect(f.ready()).toMatchObject({
      type: "error",
      duration: 8000,
      closeButton: true
    })
    expect(f.ready().action).toBeDefined()
  }
)
it("a failed later provider check preserves positive readiness and its healthy Install action", async () => {
  const f = await fixture()
  f.checks.mockRejectedValueOnce(Error("Provider unavailable"))
  await f.controller.check("stable")
  await flush()
  expect(f.ready().type).toBe("success")
  expect(f.controller.getState().readyUpdate).toBeTruthy()
  await f.ready().action!.onClick()
  await flush()
  expect(f.install).toHaveBeenCalledTimes(1)
})
// Main's failure messages are English (and an install refusal reaches the
// renderer wrapped as "Error invoking remote method ..."), so every failure
// toast shows the translated text for the failure's code instead.
it("a refused install shows the translated reason main published, not its English", async () => {
  const f = await fixture()
  f.eligible = false
  await f.ready().action!.onClick()
  await flush()
  expect(f.failure).toBeInstanceOf(Error)
  expect(f.ready().description).toBe(
    "app:_trn_update_download_unavailable_description"
  )
  expect(f.shown("updater-checking")?.description).toBe(
    "app:_trn_update_download_unavailable_description"
  )
})
it("an install failure main did not publish shows the general translated reason", async () => {
  const f = await fixture(
    vi.fn(async () => {
      throw Error("Core shutdown refused: synthetic internal detail")
    })
  )
  await f.ready().action!.onClick()
  await flush()
  expect(f.ready().description).toBe(
    "app:_trn_update_install_failed_description"
  )
})
it("an installer error during install says the install failed, not to check the connection", async () => {
  const f = await fixture()
  f.install.mockImplementation(() =>
    f.events.error(Error("Native adapter rejected"))
  )
  await f.ready().action!.onClick()
  await flush()
  expect(f.ready().description).toBe(
    "app:_trn_update_install_failed_description"
  )
})
// The core was stopped for the install and nothing starts it again, so the
// launcher cannot be used until it restarts.
it("an install that fails after the core stopped keeps a notice up whose Restart App restarts GDLauncher", async () => {
  const relaunch = vi.fn(async () => ({ status: "relaunching" }))
  const f = await fixture()
  Object.assign(window, { relaunch })
  f.install.mockImplementation(() =>
    f.events.error(Error("Native adapter rejected"))
  )
  await f.ready().action!.onClick()
  await flush()
  expect(f.controller.getState().restartRequired).toBe(true)
  const notice = f.shown("updater-checking")
  expect(notice).toMatchObject({
    type: "error",
    description: "app:_trn_update_restart_required_description",
    duration: Infinity,
    closeButton: true
  })
  expect(notice.action.label).toBe("settings:_trn_restart_app")
  await notice.action.onClick()
  expect(relaunch).toHaveBeenCalledTimes(1)
})
it("after the core stopped, a check the player asks for brings the restart notice back instead of checking", async () => {
  const f = await fixture()
  f.install.mockImplementation(() =>
    f.events.error(Error("Native adapter rejected"))
  )
  await f.ready().action!.onClick()
  await flush()
  toast.dismiss("updater-checking")
  await flush()
  expect(f.shown("updater-checking")).toBeUndefined()
  await renderer!.manualCheckForUpdates("stable")
  await flush()
  expect(f.checks).not.toHaveBeenCalled()
  expect(f.shown("updater-checking")).toMatchObject({
    type: "error",
    description: "app:_trn_update_restart_required_description"
  })
})
it("an install refused before anything stopped does not ask for a restart", async () => {
  const f = await fixture()
  f.eligible = false
  await f.ready().action!.onClick()
  await flush()
  expect(f.controller.getState().restartRequired).toBeUndefined()
  expect(f.shown("updater-checking")).toMatchObject({
    type: "error",
    duration: 8000
  })
})
it("an AppImage in a folder GDLauncher cannot write to is refused with its reason before the core stops, and Install Now stays", async () => {
  let blocked = true
  const f = await fixture(undefined, async () =>
    blocked ? "install-folder-read-only" : null
  )
  await f.ready().action!.onClick()
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  expect(f.install).not.toHaveBeenCalled()
  expect(f.ready()).toMatchObject({
    type: "error",
    description: "app:_trn_update_install_folder_read_only_description"
  })
  expect(f.ready().action).toBeDefined()
  blocked = false
  await f.ready().action!.onClick()
  await flush()
  expect(f.shutdown).toHaveBeenCalledTimes(1)
  expect(f.install).toHaveBeenCalledTimes(1)
})
