import { afterEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { EventEmitter } from "node:events"
import { toast, useSomoto } from "somoto"
import type { UpdateInfo } from "electron-updater"
import { createUpdaterController } from "../../../main/utils/updaterController"
import { ownsDownloadedUpdate } from "../../../main/utils/downloadedUpdate"
import { createOwnedInstallDispatch } from "../../../main/utils/ownedInstallDispatch"
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
let renderer: typeof import("./updater") | undefined,
  listener: ((_: unknown, state: any) => void) | undefined
let dispose: (() => void) | undefined,
  offset = 0
const flush = async () => {
  for (let i = 0; i < 40; i++) await Promise.resolve()
}
const info = (version: string): UpdateInfo => ({
  version,
  path: "fixture.zip",
  sha512: version,
  releaseDate: "2026-10-01",
  files: [{ url: "fixture.zip", sha512: version }]
})
function deferred() {
  let resolve!: (value: unknown) => void
  const promise = new Promise<unknown>((done) => {
    resolve = done
  })
  return { promise, resolve }
}
afterEach(() => {
  dispose?.()
  toast.dismiss()
  offset += 1000
})
async function fixture(
  shutdown = vi.fn(async () => {}),
  native?: EventEmitter
) {
  const events: Record<string, (...args: any[]) => void> = {},
    helper: any = {
      file: null,
      versionInfo: null,
      fileInfo: null,
      downloadedFileInfo: null
    }
  const checks = vi.fn(async (): Promise<unknown> => ({})),
    installer = vi.fn()
  const updater = {
    nativeUpdater: native,
    downloadedUpdateHelper: helper,
    channel: null,
    allowPrerelease: false,
    allowDowngrade: false,
    checkForUpdates: checks,
    quitAndInstall: installer,
    on: (event: string, cb: any) => {
      events[event] = cb
    }
  }
  const controller = createUpdaterController({
    version: "1.0.0",
    updater,
    dispatchInstall: createOwnedInstallDispatch(updater),
    artifact: () => null,
    manual: async () => ({ noUpdate: true }),
    isDownloaded: (expected) => ownsDownloadedUpdate(updater, expected),
    prepareShutdown: shutdown,
    broadcast: (state) =>
      listener?.(null, { ...state, revision: state.revision + offset })
  })
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: true }),
    getUpdateState: async () => ({
      ...controller.getState(),
      revision: controller.getState().revision + offset
    }),
    onUpdateStateChanged: (cb: any) => {
      listener = cb
    },
    installUpdate: (id: number) => controller.install(id)
  })
  renderer ??= await import("./updater")
  let store!: ReturnType<typeof useSomoto>
  createRoot((d) => {
    dispose = d
    store = useSomoto()
  })
  await flush()
  const available = (version: string) => {
    const value = info(version)
    events["update-available"](value)
    return value
  }
  const complete = async (value: UpdateInfo) => {
    Object.assign(helper, {
      file: "/synthetic/" + value.version + ".zip",
      versionInfo: value,
      fileInfo: { info: value.files[0] },
      downloadedFileInfo: { sha512: value.files[0].sha512 }
    })
    events["update-downloaded"]()
    await flush()
  }
  await complete(available("2.0.0"))
  const presentation = () =>
    store.toasts().find((t) => t.id === "updater-downloading") as any
  return {
    controller,
    events,
    helper,
    updater,
    installer,
    checks,
    shutdown,
    store,
    available,
    complete,
    presentation
  }
}
it("void install owns admission until a late error, then retry and a new positive download recover", async () => {
  const f = await fixture(),
    tokenA = f.controller.getState().readyUpdate!.id
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
  expect((await f.controller.check("stable")).status).toBe("busy")
  expect(f.checks).not.toHaveBeenCalled()
  await expect(f.controller.install(tokenA)).rejects.toThrow(
    "already in progress"
  )
  f.events.error(Error("Asynchronous installer failed"))
  await flush()
  expect(f.presentation()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(2)
  expect((await f.controller.check("beta")).status).toBe("busy")
  f.events.error(Error("Retry installer failed"))
  await flush()
  f.checks.mockImplementationOnce(async () => {
    f.available("3.0.0")
    return { downloadPromise: Promise.resolve([]) }
  })
  expect((await f.controller.check("beta")).status).toBe("checking")
  await flush()
  expect(f.presentation().type).toBe("loading")
  f.events["download-progress"]({ percent: 45 })
  await flush()
  expect(f.controller.getState().progress).toBe(45)
  await f.complete(info("3.0.0"))
  expect(ownsDownloadedUpdate(f.updater, info("3.0.0"))).toBe(true)
  expect(f.controller.getState().readyUpdate?.version).toBe("3.0.0")
  expect(f.controller.getState().readyUpdate?.id).not.toBe(tokenA)
  expect(f.presentation()).toMatchObject({ type: "success", closeButton: true })
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(3)
})
it("an already admitted check replacement is reconciled before old shutdown or native dispatch", async () => {
  const f = await fixture(),
    pending = deferred(),
    tokenA = f.controller.getState().readyUpdate!.id
  f.checks.mockImplementationOnce(() => pending.promise)
  const check = f.controller.check("beta"),
    attempt = f.presentation().action.onClick()
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  expect(f.installer).not.toHaveBeenCalled()
  expect((await f.controller.check("stable")).status).toBe("busy")
  const b = f.available("3.0.0")
  await flush()
  expect(f.presentation().type).toBe("loading")
  pending.resolve({ downloadPromise: Promise.resolve([]) })
  await check
  await attempt
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  expect(f.installer).not.toHaveBeenCalled()
  await f.complete(b)
  expect(f.presentation().type).toBe("success")
  await expect(f.controller.install(tokenA)).rejects.toThrow(
    "No automatic update"
  )
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
})
it("an already admitted no-update check settles before shutdown and preserves selected readiness", async () => {
  const f = await fixture(),
    pending = deferred(),
    token = f.controller.getState().readyUpdate!.id
  f.checks.mockImplementationOnce(() => pending.promise)
  const check = f.controller.check("stable"),
    attempt = f.presentation().action.onClick()
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  expect(f.installer).not.toHaveBeenCalled()
  f.events["update-not-available"]()
  pending.resolve({ isUpdateAvailable: false })
  await check
  await attempt
  await flush()
  expect(f.controller.getState().readyUpdate?.id).toBe(token)
  expect(f.shutdown).toHaveBeenCalledTimes(1)
  expect(f.installer).toHaveBeenCalledTimes(1)
  expect((await f.controller.check("beta")).status).toBe("busy")
})
it("all previously admitted checks settle even when an error allowed another check", async () => {
  const f = await fixture(),
    old = deferred()
  f.checks.mockImplementationOnce(() => old.promise)
  const first = f.controller.check("stable")
  f.events.error(Error("Old check failed"))
  await flush()
  f.checks.mockImplementationOnce(async () => {
    f.events["update-not-available"]()
    return {}
  })
  await f.controller.check("beta")
  expect(f.checks).toHaveBeenCalledTimes(2)
  const attempt = f.presentation().action.onClick()
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  const b = f.available("3.0.0")
  old.resolve({ downloadPromise: Promise.resolve([]) })
  await first
  await attempt
  await flush()
  expect(f.installer).not.toHaveBeenCalled()
  await f.complete(b)
  expect(f.presentation().type).toBe("success")
})
it("synchronous installer failure retains admission through dispatch and releases a real retry afterward", async () => {
  const f = await fixture()
  let reentrant: Promise<any> | undefined
  f.installer.mockImplementationOnce(() => {
    f.events.error(Error("Synchronous native rejection"))
    reentrant = f.controller.check("beta")
  })
  await f.presentation().action.onClick()
  await flush()
  expect((await reentrant!).status).toBe("busy")
  expect(f.checks).not.toHaveBeenCalled()
  expect(f.presentation()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(2)
})
it("owned shutdown rejection releases the install attempt and preserves actionable retry", async () => {
  const shutdown = vi.fn(async (): Promise<void> => {
      throw Error("Owned shutdown busy")
    }),
    f = await fixture(shutdown)
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).not.toHaveBeenCalled()
  expect(f.presentation()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  shutdown.mockResolvedValueOnce(undefined)
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
})
it("a current download error clears progress and a later ordinary check completes normally", async () => {
  const f = await fixture()
  f.checks.mockImplementationOnce(async () => {
    f.available("3.0.0")
    return {}
  })
  await f.controller.check("beta")
  await flush()
  f.events.error(Error("Current download failed"))
  await flush()
  expect(f.controller.getState().readyUpdate).toBeNull()
  expect(f.presentation()).toBeUndefined()
  f.checks.mockImplementationOnce(async () => {
    f.available("4.0.0")
    return {}
  })
  await f.controller.check("stable")
  await f.complete(info("4.0.0"))
  expect(f.checks).toHaveBeenCalledTimes(2)
  expect(f.presentation().type).toBe("success")
  expect(f.controller.getState().readyUpdate?.version).toBe("4.0.0")
})
it("failed dispatched install keeps admission until its exposed native download callbacks settle", async () => {
  const f = await fixture(),
    native = deferred()
  f.checks.mockImplementationOnce(async () => {
    f.available("2.0.0")
    return { downloadPromise: native.promise }
  })
  await f.controller.check("stable")
  await f.complete(info("2.0.0"))
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
  // The outer downloaded event permits native dispatch before this Mac-style
  // promise settles; an error must fence its later error callback and retry.
  f.events.error(Error("Native forwarding error"))
  await flush()
  expect(f.presentation()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  expect((await f.controller.check("beta")).status).toBe("busy")
  expect(f.checks).toHaveBeenCalledTimes(1)
  f.events.error(Error("Download promise error forwarding"))
  native.resolve([])
  await flush()
  f.checks.mockImplementationOnce(async () => {
    f.available("3.0.0")
    return {}
  })
  await f.controller.check("beta")
  await f.complete(info("3.0.0"))
  expect(f.controller.getState().readyUpdate?.version).toBe("3.0.0")
  expect(f.presentation().type).toBe("success")
})
it.each(["synchronous", "later"])(
  "%s failed queued native install retires only its completion callback before a successor",
  async (timing) => {
    const native = new EventEmitter(),
      normalCompletion = vi.fn(),
      obsoleteInstall = vi.fn()
    native.on("update-downloaded", normalCompletion)
    const f = await fixture(undefined, native)
    f.installer.mockImplementationOnce(() => {
      native.on("update-downloaded", obsoleteInstall)
      if (timing === "synchronous")
        f.events.error(Error("Native install rejected"))
    })
    await f.presentation().action.onClick()
    await flush()
    if (timing === "later") {
      f.events.error(Error("Native install rejected"))
      await flush()
    }
    expect(f.presentation()).toMatchObject({
      type: "error",
      duration: 8000,
      closeButton: true
    })
    expect(native.listeners("update-downloaded")).toEqual([normalCompletion])
    f.checks.mockImplementationOnce(async () => {
      f.available("3.0.0")
      return {}
    })
    await f.controller.check("beta")
    await f.complete(info("3.0.0"))
    native.emit("update-downloaded")
    expect(normalCompletion).toHaveBeenCalledTimes(1)
    expect(obsoleteInstall).not.toHaveBeenCalled()
    expect(f.controller.getState().readyUpdate?.version).toBe("3.0.0")
    expect(f.presentation().type).toBe("success")
  }
)
it("owned native listener disposal is idempotent and preserves baseline/error/unrelated listeners and installer arguments", () => {
  const native = new EventEmitter(),
    baseline = vi.fn(),
    error = vi.fn(),
    unrelated = vi.fn(),
    retired = vi.fn()
  native.on("update-downloaded", baseline)
  native.on("error", error)
  native.on("other", unrelated)
  const install = vi.fn(() => {
    native.on("update-downloaded", retired)
  })
  const dispatch = createOwnedInstallDispatch({
      nativeUpdater: native,
      quitAndInstall: install
    } as any),
    dispose = dispatch()
  expect(install).toHaveBeenCalledWith(true, true)
  dispose()
  dispose()
  native.emit("update-downloaded")
  native.emit("error", Error("Synthetic"))
  native.emit("other")
  expect(retired).not.toHaveBeenCalled()
  expect(baseline).toHaveBeenCalledTimes(1)
  expect(error).toHaveBeenCalledTimes(1)
  expect(unrelated).toHaveBeenCalledTimes(1)
  const plain = vi.fn(),
    release = createOwnedInstallDispatch({ quitAndInstall: plain })()
  release()
  release()
  expect(plain).toHaveBeenCalledWith(true, true)
})
it("a thrown native dispatch removes its attached completion action and preserves preexisting listeners", () => {
  const native = new EventEmitter(),
    baseline = vi.fn(),
    retired = vi.fn()
  native.on("update-downloaded", baseline)
  const dispatch = createOwnedInstallDispatch({
    nativeUpdater: native,
    quitAndInstall: () => {
      native.on("update-downloaded", retired)
      throw Error("Native dispatch threw")
    }
  } as any)
  expect(dispatch).toThrow("Native dispatch threw")
  native.emit("update-downloaded")
  expect(baseline).toHaveBeenCalledTimes(1)
  expect(retired).not.toHaveBeenCalled()
})
it("a thrown dispatch retains admission until its known native download callbacks settle", async () => {
  const f = await fixture(),
    native = deferred()
  f.checks.mockImplementationOnce(async () => {
    f.available("2.0.0")
    return { downloadPromise: native.promise }
  })
  await f.controller.check("stable")
  await f.complete(info("2.0.0"))
  f.installer.mockImplementationOnce(() => {
    throw Error("Native dispatch threw")
  })
  await f.presentation().action.onClick()
  await flush()
  expect(f.presentation()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  expect((await f.controller.check("beta")).status).toBe("busy")
  expect(f.checks).toHaveBeenCalledTimes(1)
  f.events.error(Error("Known native callback failed"))
  native.resolve([])
  await flush()
  f.checks.mockImplementationOnce(async () => {
    f.available("3.0.0")
    return {}
  })
  await f.controller.check("beta")
  await f.complete(info("3.0.0"))
  expect(f.controller.getState().readyUpdate?.version).toBe("3.0.0")
  expect(f.presentation().type).toBe("success")
})
it("manual ready reopen preserves a pending install presentation and real error restores retry", async () => {
  const f = await fixture()
  toast.dismiss("updater-downloading")
  await renderer!.manualCheckForUpdates("stable")
  await flush()
  expect(f.presentation().type).toBe("success")
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
  await renderer!.manualCheckForUpdates("stable")
  await flush()
  expect(f.presentation()).toMatchObject({
    type: "loading",
    duration: Infinity,
    closeButton: false
  })
  expect(f.presentation().action).toBeUndefined()
  f.events.error(Error("Asynchronous install failed"))
  await flush()
  expect(f.presentation()).toMatchObject({
    type: "error",
    duration: 8000,
    closeButton: true
  })
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(2)
})

/** The 30-minute check finding `version` again, served from its cached file. */
async function repeatCheck(
  f: Awaited<ReturnType<typeof fixture>>,
  version: string
) {
  f.checks.mockImplementationOnce(async () => {
    await f.complete(f.available(version))
    return {}
  })
  await f.controller.check("stable")
  await flush()
}
/** Every state the window is sent from here on. */
function sentStates() {
  const sent: any[] = [],
    inner = listener!
  listener = (event, state) => {
    sent.push(state)
    inner(event, state)
  }
  return sent
}
it("Later stays closed when a later check finds the same update: no download notice, the badge stays and Install still works", async () => {
  const f = await fixture(),
    token = f.controller.getState().readyUpdate!.id
  expect(f.presentation().type).toBe("success")
  toast.dismiss("updater-downloading")
  await flush()
  expect(f.presentation()).toBeUndefined()
  const sent = sentStates()
  await repeatCheck(f, "2.0.0")
  await repeatCheck(f, "2.0.0")
  expect(sent.map((state) => state.state)).toEqual([
    "checking",
    "downloading",
    "downloaded",
    "checking",
    "downloading",
    "downloaded"
  ])
  // The update stayed ready under its id through both repeats.
  expect(sent.map((state) => state.readyUpdate?.id)).toEqual(
    Array(6).fill(token)
  )
  expect(f.presentation()).toBeUndefined()
  expect(renderer!.hasPendingUpdate()).toBe(true)
  expect(renderer!.pendingUpdateVersion()).toBe("2.0.0")
  // A check the player asks for brings the notice back, and it installs.
  await renderer!.manualCheckForUpdates("stable")
  await flush()
  expect(f.presentation().type).toBe("success")
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
})
it("a newer update found by a later check replaces the closed one and is announced", async () => {
  const f = await fixture(),
    token = f.controller.getState().readyUpdate!.id
  toast.dismiss("updater-downloading")
  await flush()
  await repeatCheck(f, "2.1.0")
  const ready = f.controller.getState().readyUpdate!
  expect(ready.version).toBe("2.1.0")
  expect(ready.id).not.toBe(token)
  expect(f.presentation().type).toBe("success")
  await expect(f.controller.install(token)).rejects.toThrow(
    "No automatic update is ready to install"
  )
})
it("an Install clicked while a later check repeats the same update goes through with it", async () => {
  const f = await fixture(),
    token = f.controller.getState().readyUpdate!.id,
    repeat = deferred()
  f.checks.mockImplementationOnce(async () => {
    const value = f.available("2.0.0")
    await repeat.promise
    await f.complete(value)
    return {}
  })
  const checking = f.controller.check("stable")
  await flush()
  const installing = f.controller.install(token)
  await flush()
  expect(f.shutdown).not.toHaveBeenCalled()
  repeat.resolve(undefined)
  await checking
  await installing
  expect(f.shutdown).toHaveBeenCalledTimes(1)
  expect(f.installer).toHaveBeenCalledTimes(1)
})
it("a repeat that fails after replacing the cached file takes the ready update with it", async () => {
  const f = await fixture()
  f.checks.mockImplementationOnce(async () => {
    f.available("2.0.0")
    // electron-updater found the cached file unusable and cleared it.
    Object.assign(f.helper, {
      file: null,
      versionInfo: null,
      fileInfo: null,
      downloadedFileInfo: null
    })
    f.events.error(Error("Download failed"))
    return {}
  })
  await f.controller.check("stable")
  await flush()
  expect(f.controller.getState()).toMatchObject({
    state: "error",
    readyUpdate: null
  })
  expect(renderer!.hasPendingUpdate()).toBe(false)
})
it("a failed check leaves a ready update whose file is still there", async () => {
  const f = await fixture(),
    token = f.controller.getState().readyUpdate!.id
  f.checks.mockImplementationOnce(async () => {
    f.available("2.0.0")
    f.events.error(Error("Feed went away"))
    return {}
  })
  await f.controller.check("stable")
  await flush()
  expect(f.controller.getState().readyUpdate?.id).toBe(token)
  expect(renderer!.hasPendingUpdate()).toBe(true)
})
it("the same version published again with another file is a new update, not a repeat", async () => {
  const f = await fixture(),
    token = f.controller.getState().readyUpdate!.id
  toast.dismiss("updater-downloading")
  await flush()
  const rebuilt: UpdateInfo = {
    ...info("2.0.0"),
    sha512: "rebuilt",
    files: [{ url: "fixture.zip", sha512: "rebuilt" }]
  }
  f.checks.mockImplementationOnce(async () => {
    f.events["update-available"](rebuilt)
    await f.complete(rebuilt)
    return {}
  })
  const sent = sentStates()
  await f.controller.check("stable")
  await flush()
  expect(
    sent.find((state) => state.state === "downloading").readyUpdate
  ).toBeNull()
  expect(f.controller.getState().readyUpdate!.id).not.toBe(token)
  expect(f.presentation().type).toBe("success")
})
it("a repeat on macOS waits on a fresh native preparation and leaves no old watcher behind", async () => {
  const native = new EventEmitter(),
    f = await fixture(undefined, native),
    token = f.controller.getState().readyUpdate!.id
  // What MacUpdater carries, and what makes its preparation watched.
  Object.assign(f.updater, {
    autoInstallOnAppQuit: true,
    squirrelDownloadedUpdate: false
  })
  const settled = {
    error: native.listenerCount("error"),
    completed: native.listenerCount("update-downloaded")
  }
  const counts = () => ({
    error: native.listenerCount("error"),
    completed: native.listenerCount("update-downloaded")
  })
  await repeatCheck(f, "2.0.0")
  // Squirrel is fed again: one watcher, for the repeat.
  expect(counts()).toEqual({
    error: settled.error + 1,
    completed: settled.completed + 1
  })
  expect(f.controller.getState().readyUpdate!.id).toBe(token)
  native.emit("update-downloaded")
  await flush()
  expect(counts()).toEqual(settled)
  await repeatCheck(f, "2.0.0")
  // The repeat's own native failure is the one that counts.
  native.emit("error", Error("Squirrel failed"))
  await flush()
  expect(f.controller.getState()).toMatchObject({
    state: "error",
    readyUpdate: null,
    error: { code: "prepare-failed" }
  })
  expect(counts()).toEqual(settled)
})
it("an update whose install failed is announced again when a later check downloads it", async () => {
  const f = await fixture(),
    token = f.controller.getState().readyUpdate!.id
  await f.presentation().action.onClick()
  await flush()
  expect(f.installer).toHaveBeenCalledTimes(1)
  f.events.error(Error("Asynchronous install failed"))
  await flush()
  expect(f.controller.getState().readyUpdate!.id).toBe(token)
  toast.dismiss("updater-downloading")
  await flush()
  await repeatCheck(f, "2.0.0")
  const ready = f.controller.getState().readyUpdate!
  expect(ready.version).toBe("2.0.0")
  expect(ready.id).not.toBe(token)
  expect(f.presentation().type).toBe("success")
})
