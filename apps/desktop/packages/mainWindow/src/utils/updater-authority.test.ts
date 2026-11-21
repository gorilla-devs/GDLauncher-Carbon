import { expect, it, vi } from "vitest"
import { createUpdaterController } from "../../../main/utils/updaterController"
import { ownsDownloadedUpdate } from "../../../main/utils/downloadedUpdate"
import type { UpdateInfo } from "electron-updater"
import { installReadyUpdateAction } from "../../../preload/installReadyUpdate"
const info = (version = "2.0.0", sha512 = "owned"): UpdateInfo => ({
  version,
  releaseDate: "2026-10-01",
  path: "fixture.zip",
  sha512,
  files: [{ url: "fixture.zip", sha512 }]
})
function fixture(version = "1.0.0", shutdown = vi.fn(async () => {})) {
  const events: Record<string, (...args: any[]) => void> = {},
    install = vi.fn(),
    check = vi.fn(async () => ({}))
  const helper: any = {
    file: null,
    versionInfo: null,
    fileInfo: null,
    downloadedFileInfo: null
  }
  const updater = {
    downloadedUpdateHelper: helper,
    channel: null,
    allowPrerelease: false,
    allowDowngrade: false,
    checkForUpdates: check,
    quitAndInstall: install,
    on: (name: string, callback: any) => {
      events[name] = callback
    }
  }
  const controller = createUpdaterController({
    version,
    updater,
    artifact: () => null,
    manual: async () => ({ noUpdate: true }),
    isDownloaded: (expected) => ownsDownloadedUpdate(updater, expected),
    broadcast: () => {},
    prepareShutdown: shutdown
  })
  const download = (value = info()) => {
    events["update-available"](value)
    Object.assign(helper, {
      file: "/synthetic/fixture.zip",
      versionInfo: value,
      fileInfo: { info: value.files[0] },
      downloadedFileInfo: { sha512: value.files[0].sha512 }
    })
    events["update-downloaded"]()
    return controller.getState().readyUpdate!.id
  }
  return {
    events,
    install,
    check,
    helper,
    updater,
    controller,
    shutdown,
    download
  }
}
it("pinned downloaded metadata guard checks helper file, version and exact selected checksum without native IO", () => {
  const f = fixture()
  f.download()
  expect(ownsDownloadedUpdate(f.updater, info())).toBe(true)
  expect(ownsDownloadedUpdate(f.updater, info("2.0.0", "replacement"))).toBe(
    false
  )
  expect(ownsDownloadedUpdate(f.updater, info("3.0.0"))).toBe(false)
  f.helper.downloadedFileInfo = null
  expect(ownsDownloadedUpdate(f.updater, info())).toBe(false)
  f.helper.downloadedFileInfo = { sha512: "owned" }
  f.helper.file = null
  expect(ownsDownloadedUpdate(f.updater, info())).toBe(false)
})
it("genuine automatic download keeps check lock until completed, then supports the existing second check", async () => {
  const f = fixture()
  await f.controller.check("stable")
  f.events["update-available"](info())
  expect((await f.controller.check("beta")).status).toBe("busy")
  const token = f.download()
  await f.controller.check("stable")
  expect(f.check).toHaveBeenCalledTimes(2)
  f.events["update-not-available"]()
  expect(f.controller.getState().readyUpdate?.id).toBe(token)
  await f.controller.install(token)
  expect(f.shutdown).toHaveBeenCalledTimes(1)
  expect(f.install).toHaveBeenCalledTimes(1)
})
it("snapshot/manual-only checks retain policy and bare downloaded phase cannot authorize install", async () => {
  const f = fixture("1.0.0-snapshot")
  expect((await f.controller.check("stable")).status).toBe("snapshot-version")
  expect(f.check).not.toHaveBeenCalled()
  f.events["update-downloaded"]()
  await expect(f.controller.install()).rejects.toThrow("No automatic update")
  expect(f.shutdown).not.toHaveBeenCalled()
})
it("explicit channel downgrade remains supported by positive downloaded ownership", async () => {
  const f = fixture("3.0.0-beta")
  await f.controller.check("stable")
  expect(f.updater.allowDowngrade).toBe(true)
  const token = f.download(info("2.0.0"))
  await f.controller.install(token)
  expect(f.install).toHaveBeenCalledTimes(1)
})
it("replacement during awaited shutdown cannot install under the old token and refuses concurrent install", async () => {
  let finish!: () => void
  const f = fixture(
    "1.0.0",
    vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        })
    )
  )
  const token = f.download(),
    attempt = f.controller.install(token)
  await expect(f.controller.install(token)).rejects.toThrow(
    "already in progress"
  )
  expect((await f.controller.check("stable")).status).toBe("busy")
  const next = f.download(info("3.0.0", "next"))
  finish()
  await expect(attempt).rejects.toThrow("changed during shutdown")
  expect(f.install).not.toHaveBeenCalled()
  expect(f.controller.getState().readyUpdate?.id).toBe(next)
  await expect(f.controller.install(token)).rejects.toThrow(
    "No automatic update"
  )
})
it("helper loss after awaited shutdown retires readiness without invoking native installer", async () => {
  let finish!: () => void
  const f = fixture(
    "1.0.0",
    vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        })
    )
  )
  const token = f.download(),
    attempt = f.controller.install(token)
  f.helper.file = null
  finish()
  await expect(attempt).rejects.toThrow("changed during shutdown")
  expect(f.install).not.toHaveBeenCalled()
  expect(f.controller.getState().readyUpdate).toBeNull()
})
it("fatal-page install consumer captures positive token and renders a caught owned-shutdown rejection", async () => {
  const f = fixture(
      "1.0.0",
      vi.fn(async () => {
        throw Error("Owned runtime busy")
      })
    ),
    failed = vi.fn()
  f.download()
  const action = installReadyUpdateAction(
    () => f.controller.getState().readyUpdate?.id,
    (id) => f.controller.install(id),
    failed
  )
  await action()
  expect(failed).toHaveBeenCalledWith("Owned runtime busy")
  expect(f.install).not.toHaveBeenCalled()
})
it("fatal-page rejection from an obsolete attempt does not replace the newer update presentation", async () => {
  let fail!: (error: Error) => void,
    id = 1
  const failed = vi.fn(),
    invoke = vi.fn(
      () =>
        new Promise<void>((_, reject) => {
          fail = reject
        })
    )
  const action = installReadyUpdateAction(() => id, invoke, failed),
    pending = action()
  expect(invoke).toHaveBeenCalledWith(1)
  id = 2
  fail(Error("Old failure"))
  await pending
  expect(failed).not.toHaveBeenCalled()
})
