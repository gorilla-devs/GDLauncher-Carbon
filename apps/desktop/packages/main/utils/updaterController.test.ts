import { describe, expect, it, vi } from "vitest"
import { EventEmitter } from "node:events"
import type { ChildProcess } from "node:child_process"
import {
  AppImageUpdater,
  type AppUpdater,
  type UpdateInfo
} from "electron-updater"
import { GenericProvider } from "electron-updater/out/providers/GenericProvider"
import { createUpdaterController } from "./updaterController"
import { activeUpdateMetadata } from "./updateProvider"
import { selectUpdateArtifact } from "./updateArtifact"
import type { UpdateStateData } from "./updateState"
import { ShutdownBlockedError, stopCoreForShutdown } from "./coreShutdown"
import { ShutdownFlow, type ShutdownChoice } from "./shutdownFlow"
function controller(
  check: () => Promise<unknown> = async () => null,
  manual: () => Promise<any> = async () => null,
  version = "1.0.0"
) {
  const events: Record<string, (...args: any[]) => void> = {},
    states: UpdateStateData[] = []
  let checks = 0,
    installed = 0,
    shutdowns = 0
  const updater = {
    channel: null as string | null,
    allowPrerelease: false,
    allowDowngrade: false,
    checkForUpdates: async () => {
      checks++
      return check()
    },
    on: (e: string, f: (...args: any[]) => void) => {
      events[e] = f
    },
    quitAndInstall: () => {
      installed++
    }
  }
  const wrapper = createUpdaterController({
    version,
    updater,
    broadcast: (s) => states.push(s),
    artifact: () => "https://active.invalid/update.zip",
    manual,
    isDownloaded: () => true,
    prepareShutdown: async () => {
      shutdowns++
    }
  })
  return {
    wrapper,
    updater,
    events,
    states,
    get checks() {
      return checks
    },
    get installed() {
      return installed
    },
    get shutdowns() {
      return shutdowns
    }
  }
}
const info = {
  version: "2.0.0",
  releaseDate: "2026-10-01",
  files: [
    { url: "GDLauncher__2.0.0__linux__x64.zip", sha512: "synthetic-checksum" }
  ]
} as UpdateInfo
const installation = {
  platform: "linux" as const,
  arch: "x64",
  format: "zip" as const
}
describe("Task23 updater settlement", () => {
  it("null updater clears checking payloads, broadcasts unsupported and permits retry", async () => {
    const value = controller()
    expect((await value.wrapper.check("stable")).status).toBe("unsupported")
    expect(value.wrapper.getState()).toMatchObject({
      state: "unsupported",
      updateInfo: null,
      progress: 0
    })
    expect((await value.wrapper.check("stable")).status).toBe("unsupported")
    expect(value.checks).toBe(2)
    expect(value.states.at(-1)?.error?.message).toContain("retry")
  })
  it("pinned packaged AppImage itself returns null without any request/download", async () => {
    const previous = process.env.APPIMAGE
    delete process.env.APPIMAGE
    try {
      const app = {
        isPackaged: true,
        version: "1.0.0",
        name: "Fixture",
        appUpdateConfigPath: "/synthetic-unused",
        userDataPath: "/synthetic-unused"
      }
      const updater = new AppImageUpdater(undefined, app as any)
      updater.logger = { info() {}, warn() {}, error() {}, debug() {} }
      ;(updater as any).httpExecutor = {
        request: () => {
          throw Error("transport must not run")
        }
      } as any
      expect(await updater.checkForUpdates()).toBeNull()
    } finally {
      if (previous === undefined) delete process.env.APPIMAGE
      else process.env.APPIMAGE = previous
    }
  })
  it("snapshot returns before provider check", async () => {
    const value = controller(undefined, undefined, "1.0.0-snapshot")
    expect((await value.wrapper.check("stable")).status).toBe(
      "snapshot-version"
    )
    expect(value.checks).toBe(0)
  })
  it("real automatic download retains lock until downloaded/error; no-update clears payload", async () => {
    const value = controller(async () => ({ updateInfo: info }))
    await value.wrapper.check("stable")
    value.events["update-available"](info)
    expect((await value.wrapper.check("stable")).status).toBe("busy")
    value.events["download-progress"]({ percent: 30 })
    expect(value.wrapper.getState().progress).toBe(30)
    value.events["update-downloaded"]()
    await value.wrapper.check("stable")
    expect(value.checks).toBe(2)
    value.events.error(Error("synthetic"))
    await value.wrapper.check("stable")
    expect(value.checks).toBe(3)
    value.events["update-not-available"]()
    expect(value.wrapper.getState()).toMatchObject({
      state: "no-update",
      updateInfo: null,
      progress: 0
    })
  })
  it("rejection releases and retry metadata-only manual never installs", async () => {
    let failed = true
    const value = controller(
      async () => {
        if (failed) throw Error("synthetic")
        return null
      },
      async () => ({
        ...info,
        downloadUrl: "https://active.invalid/fixture.zip"
      })
    )
    expect((await value.wrapper.check("stable")).status).toBe("error")
    failed = false
    expect((await value.wrapper.check("beta")).status).toBe("manual-update")
    expect(value.updater.channel).toBe("beta")
    expect(value.updater.allowPrerelease).toBe(true)
    expect(value.installed).toBe(0)
    expect(value.shutdowns).toBe(0)
  })
  it("stale manual completion after error/new check cannot overwrite current download", async () => {
    let finish: (result: any) => void = () => {}
    let check = 0
    const value = controller(
      async () => (++check === 1 ? null : {}),
      () =>
        new Promise((resolve) => {
          finish = resolve
        })
    )
    const first = value.wrapper.check("stable")
    await Promise.resolve()
    await Promise.resolve()
    value.events.error(Error("synthetic"))
    await value.wrapper.check("beta")
    value.events["update-available"](info)
    finish({ ...info, downloadUrl: "https://stale.invalid/fixture.zip" })
    await first
    expect(value.wrapper.getState().state).toBe("downloading")
    expect((await value.wrapper.check("stable")).status).toBe("busy")
  })
  it("matching current manual metadata is truthful no-update; unsupported install refuses", async () => {
    const value = controller(undefined, async () => ({ noUpdate: true }))
    expect((await value.wrapper.check("stable")).status).toBe("no-update")
    await expect(value.wrapper.install()).rejects.toThrow("No automatic update")
    expect(value.installed).toBe(0)
  })
  it("install awaits owned shutdown before injected installer", async () => {
    const value = controller()
    value.events["update-available"](info)
    value.events["update-downloaded"]()
    await value.wrapper.install()
    expect(value.shutdowns).toBe(1)
    expect(value.installed).toBe(1)
  })
  // main/index.ts wires prepareShutdown to prepareOwnedShutdown("update"),
  // which runs this handshake. Whether leftover records from an earlier
  // session hold it up is the core's answer (api/runtime.rs
  // an_update_install_commits_despite_leftover_process_records).
  it("install asks the core for an 'update' shutdown and installs only after the core exited", async () => {
    const order: string[] = []
    const core = Object.assign(new EventEmitter(), {
      exitCode: null,
      signalCode: null,
      kill: vi.fn(() => {
        order.push("core exit")
        core.emit("exit", 0)
        return true
      })
    })
    const transport = vi.fn(async (url: string, options: any) => {
      const body = JSON.parse(options.body)
      order.push(`${String(url).split("/").at(-1)} ${body.purpose}`)
      return {
        ok: true,
        json: async () => ({
          ...body,
          runtimePath: process.cwd(),
          status: { ...body, phase: "ready", blockers: [] }
        })
      }
    })
    const events: Record<string, (...args: any[]) => void> = {}
    const wrapper = createUpdaterController({
      version: "1.0.0",
      updater: {
        channel: null as string | null,
        allowPrerelease: false,
        allowDowngrade: false,
        checkForUpdates: async () => null,
        on: (e: string, f: (...args: any[]) => void) => {
          events[e] = f
        },
        quitAndInstall: () => {
          order.push("install")
        }
      },
      broadcast: () => {},
      artifact: () => "https://active.invalid/update.zip",
      manual: async () => null,
      isDownloaded: () => true,
      prepareShutdown: () =>
        stopCoreForShutdown(
          {
            port: 1,
            apiToken: "fixture",
            process: core as unknown as ChildProcess
          },
          "update",
          transport as any
        )
    })
    events["update-available"](info)
    events["update-downloaded"]()
    await wrapper.install()
    expect(order).toEqual([
      "prepare-shutdown update",
      "shutdown update",
      "core exit",
      "install"
    ])
  })
})
// main/index.ts runs prepareShutdown as ownedShutdown("update"): this flow.
describe("an update install the core cannot stop for", () => {
  function refused(answers: ShutdownChoice[]) {
    const order: string[] = []
    const events: Record<string, (...args: any[]) => void> = {}
    let blocked = true
    const wrapper = createUpdaterController({
      version: "1.0.0",
      updater: {
        channel: null as string | null,
        allowPrerelease: false,
        allowDowngrade: false,
        checkForUpdates: async () => null,
        on: (e: string, f: (...args: any[]) => void) => {
          events[e] = f
        },
        quitAndInstall: () => {
          order.push("quitAndInstall")
        }
      },
      broadcast: () => {},
      artifact: () => "https://active.invalid/update.zip",
      manual: async () => null,
      isDownloaded: () => true,
      prepareShutdown: () =>
        new ShutdownFlow("update", {
          stop: async () => {
            order.push("prepare-shutdown")
            if (blocked)
              throw new ShutdownBlockedError("busy", "unresolved", [
                { kind: "server", name: "server 7 run" }
              ])
          },
          cancel: async () => {
            order.push("cancel-shutdown")
            return "released"
          },
          force: async () => {
            order.push("force")
          },
          prompt: async (view) => {
            order.push(`prompt ${view.action} ${view.stage}`)
            return answers.shift() ?? "keep"
          }
        }).run()
    })
    events["update-available"](info)
    events["update-downloaded"]()
    return {
      wrapper,
      order,
      unblock: () => {
        blocked = false
      }
    }
  }
  it("Install now ends the core and then installs", async () => {
    const value = refused(["force"])
    await expect(value.wrapper.install()).resolves.toBeUndefined()
    expect(value.order).toEqual([
      "prepare-shutdown",
      "prompt update blocked",
      // The window says it is stopping until the core's exit is seen.
      "prompt update stopping",
      "force",
      "quitAndInstall"
    ])
  })
  it("Keep GDLauncher open gives the core back and the same update can be installed later", async () => {
    const value = refused(["keep"])
    const ready = value.wrapper.getState().readyUpdate
    expect(await value.wrapper.install(ready!.id)).toBe("kept")
    expect(value.order).toEqual([
      "prepare-shutdown",
      "prompt update blocked",
      "cancel-shutdown"
    ])
    expect(value.wrapper.getState()).toMatchObject({
      state: "downloaded",
      readyUpdate: ready
    })
    value.unblock()
    await value.wrapper.install(ready!.id)
    expect(value.order.slice(3)).toEqual(["prepare-shutdown", "quitAndInstall"])
  })
})
function provider(base: string, channel = "latest") {
  return new GenericProvider(
    { provider: "generic", url: base },
    { channel } as AppUpdater,
    {
      platform: "linux",
      executor: {
        request() {
          throw Error("no HTTP authorized")
        }
      } as any,
      isUseMultipleRangeRequest: false
    }
  )
}
describe("Task23 active-provider matching metadata", () => {
  it("active feed resolves relative/absolute metadata with folders and no launch publish env", () => {
    for (const base of [
      "https://packaged.invalid/folder",
      "https://packaged.invalid/folder/"
    ]) {
      const active = provider(base),
        updater = {
          updateInfoAndProvider: { info, provider: active }
        } as unknown as AppUpdater
      expect(
        activeUpdateMetadata(updater, installation, "1.0.0").artifact(info)
      ).toBe(
        "https://packaged.invalid/folder/GDLauncher__2.0.0__linux__x64.zip"
      )
      const absolute = {
        ...info,
        files: [
          {
            url: "https://artifact.invalid/GDLauncher__2.0.0__linux__x64.zip",
            sha512: "synthetic"
          }
        ]
      }
      expect(
        activeUpdateMetadata(
          {
            updateInfoAndProvider: { info: absolute, provider: active }
          } as unknown as AppUpdater,
          installation,
          "1.0.0"
        ).artifact(absolute)
      ).toBe("https://artifact.invalid/GDLauncher__2.0.0__linux__x64.zip")
    }
  })
  it("override wins and latest/beta/alpha follow actual provider channel", async () => {
    for (const channel of ["latest", "beta", "alpha"]) {
      const active = provider("https://override.invalid/feed/", channel)
      // electron-updater adds the architecture to Linux channels other than x64.
      const arch = process.arch === "x64" ? "" : `-${process.arch}`
      expect((active as any).channel).toBe(`${channel}-linux${arch}`)
      let checks = 0
      active.getLatestVersion = async () => {
        checks++
        return info
      }
      const updater = {
        clientPromise: Promise.resolve(active),
        configOnDisk: {
          get value() {
            throw Error("override must win")
          }
        }
      } as unknown as AppUpdater
      expect(
        await activeUpdateMetadata(updater, installation, "1.0.0").manual()
      ).toMatchObject({
        downloadUrl:
          "https://override.invalid/feed/GDLauncher__2.0.0__linux__x64.zip"
      })
      expect(checks).toBe(1)
    }
  })
  it("selects known format/platform/arch only; missing/ambiguous identity has no fabricated link", () => {
    const active = provider("https://feed.invalid/")
    const mixed = {
      ...info,
      files: ["AppImage", "zip", "deb", "rpm"]
        .map((ext) => ({
          url: `GDLauncher__2.0.0__linux__x64.${ext}`,
          sha512: "synthetic"
        }))
        .concat([
          { url: "GDLauncher__2.0.0__linux__arm64.zip", sha512: "synthetic" }
        ])
    }
    const files = active.resolveFiles(mixed)
    for (const format of ["appimage", "zip", "deb", "rpm"] as const)
      expect(
        selectUpdateArtifact(files, { ...installation, format })
      ).toContain(format === "appimage" ? ".AppImage" : "." + format)
    expect(
      selectUpdateArtifact(files, { ...installation, format: null })
    ).toBeNull()
    expect(
      selectUpdateArtifact(files, { ...installation, platform: "win32" })
    ).toBeNull()
    expect(
      selectUpdateArtifact(
        files.concat([
          {
            ...files[1],
            url: new URL(
              "https://other.invalid/GDLauncher__2.0.0__linux__x64.zip"
            )
          }
        ]),
        installation
      )
    ).toBeNull()
    expect(
      selectUpdateArtifact(
        active.resolveFiles({
          ...info,
          files: [{ url: "unknown.zip", sha512: "synthetic" }]
        }),
        installation
      )
    ).toBeNull()
  })
})
// The renderer translates failures by code; main's message is English only.
describe("updater failures carry a translatable code", () => {
  const noDownload = async (): Promise<null> => null
  const metadataFailure = async (): Promise<null> => {
    throw Error("synthetic metadata failure")
  }
  it.each([
    ["no manual download", noDownload, "no-manual-download"],
    ["manual metadata failure", metadataFailure, "manual-check-failed"]
  ] as const)(
    "%s publishes unsupported with its code",
    async (_, manual, code) => {
      const value = controller(async () => null, manual)
      await value.wrapper.check("stable")
      expect(value.wrapper.getState()).toMatchObject({
        state: "unsupported",
        error: { code }
      })
    }
  )
  it("a failed provider check publishes check-failed", async () => {
    const value = controller(async () => {
      throw Error("synthetic provider failure")
    })
    await value.wrapper.check("stable")
    expect(value.wrapper.getState().error?.code).toBe("check-failed")
  })
  it("an updater error event publishes update-failed", () => {
    const value = controller()
    value.events.error(Error("synthetic download failure"))
    expect(value.wrapper.getState().error?.code).toBe("update-failed")
  })
  it("a downloaded event without its download publishes download-unavailable", () => {
    const value = controller()
    value.events["update-downloaded"]()
    expect(value.wrapper.getState().error?.code).toBe("download-unavailable")
  })
})
// Linux installs without APPIMAGE or a package-type marker (nixpkgs, AUR
// repackages, unpacked AppImages, self-built zips) have no active
// electron-updater, so every check reads the channel's metadata instead.
describe("an installation that cannot update itself", () => {
  const latest = {
    version: "2.0.40",
    releaseDate: "2026-10-01",
    files: ["AppImage", "deb", "rpm"].map((ext) => ({
      url: `GDLauncher__2.0.40__linux__x64.${ext}`,
      sha512: "synthetic"
    }))
  } as UpdateInfo
  function channel(published: UpdateInfo) {
    const active = provider("https://feed.invalid/")
    active.getLatestVersion = async () => published
    return { clientPromise: Promise.resolve(active) } as unknown as AppUpdater
  }
  it.each(["zip", null] as const)(
    "is up to date when it runs the latest version, though the channel has no %s download",
    async (format) => {
      const metadata = activeUpdateMetadata(
        channel(latest),
        { ...installation, format },
        "2.0.40"
      )
      expect(await metadata.manual()).toEqual({ noUpdate: true })
      const value = controller(
        async () => null,
        () => metadata.manual()
      )
      expect((await value.wrapper.check("stable")).status).toBe("no-update")
      expect(value.wrapper.getState()).toMatchObject({
        state: "no-update",
        error: null
      })
    }
  )
  it("names the newer version when the channel has no download for it", async () => {
    const metadata = activeUpdateMetadata(
      channel({ ...latest, version: "2.0.41" }),
      installation,
      "2.0.40"
    )
    const value = controller(
      async () => null,
      () => metadata.manual()
    )
    expect((await value.wrapper.check("stable")).status).toBe("unsupported")
    expect(value.wrapper.getState()).toMatchObject({
      state: "unsupported",
      updateInfo: { version: "2.0.41", downloadUrl: null },
      error: { code: "no-manual-download" }
    })
  })
  it("still links the matching download of a newer version", async () => {
    const metadata = activeUpdateMetadata(
      channel({ ...latest, version: "2.0.41" }),
      { ...installation, format: "deb" },
      "2.0.40"
    )
    expect(await metadata.manual()).toMatchObject({
      version: "2.0.41",
      downloadUrl: "https://feed.invalid/GDLauncher__2.0.40__linux__x64.deb"
    })
  })
  it("offers an older version only when the channel switch allows a downgrade", async () => {
    const older = channel({ ...latest, version: "2.0.39" })
    expect(
      await activeUpdateMetadata(older, installation, "2.0.40").manual()
    ).toEqual({ noUpdate: true })
    ;(older as { allowDowngrade: boolean }).allowDowngrade = true
    expect(
      await activeUpdateMetadata(
        older,
        { ...installation, format: "deb" },
        "2.0.40"
      ).manual()
    ).toMatchObject({ version: "2.0.39" })
  })
})
describe("an update install that fails leaves GDLauncher usable or says to restart", () => {
  function installable(
    options: {
      native?: EventEmitter
      blocker?: () => Promise<any>
      shutdown?: () => Promise<any>
      quitAndInstall?: () => void
    } = {}
  ) {
    const events: Record<string, (...args: any[]) => void> = {}
    const order: string[] = []
    const wrapper = createUpdaterController({
      version: "1.0.0",
      updater: {
        channel: null as string | null,
        allowPrerelease: false,
        allowDowngrade: false,
        checkForUpdates: async () => null,
        on: (e: string, f: (...args: any[]) => void) => {
          events[e] = f
        },
        quitAndInstall: () => {
          order.push("install")
          options.quitAndInstall?.()
        },
        // Squirrel.Mac's emitter, as electron-updater's MacUpdater has it.
        ...(options.native
          ? {
              nativeUpdater: options.native,
              autoInstallOnAppQuit: true,
              squirrelDownloadedUpdate: false
            }
          : {})
      } as any,
      broadcast: () => {},
      artifact: () => null,
      manual: async () => ({ noUpdate: true }),
      isDownloaded: () => true,
      installBlocker: options.blocker,
      prepareShutdown: async () => {
        order.push("core stopped")
        return options.shutdown ? options.shutdown() : "stopped"
      }
    })
    events["update-available"](info)
    events["update-downloaded"]()
    return { wrapper, events, order }
  }
  const settle = async () => {
    for (let i = 0; i < 20; i++) await Promise.resolve()
  }
  it("waits for Squirrel.Mac to take the update before stopping the core", async () => {
    const native = new EventEmitter()
    const value = installable({ native })
    const installing = value.wrapper.install()
    await settle()
    expect(value.order).toEqual([])
    native.emit("update-downloaded")
    await installing
    expect(value.order).toEqual(["core stopped", "install"])
    expect(value.wrapper.getState().restartRequired).toBeUndefined()
  })
  it("keeps the core running when Squirrel.Mac fails while the install waits for it", async () => {
    const native = new EventEmitter()
    const value = installable({ native })
    const installing = value.wrapper.install()
    await settle()
    native.emit("error", Error("Squirrel could not verify the update"))
    await expect(installing).rejects.toThrow()
    expect(value.order).toEqual([])
    expect(value.wrapper.getState()).toMatchObject({
      state: "error",
      readyUpdate: null,
      error: { code: "prepare-failed" }
    })
    expect(value.wrapper.getState().restartRequired).toBeUndefined()
  })
  it("refuses an AppImage it cannot replace before stopping the core and keeps the update ready", async () => {
    let blocked = true
    const value = installable({
      blocker: async () => (blocked ? "install-folder-read-only" : null)
    })
    const ready = value.wrapper.getState().readyUpdate!
    await expect(value.wrapper.install(ready.id)).rejects.toThrow(
      "cannot write to the folder"
    )
    expect(value.order).toEqual([])
    expect(value.wrapper.getState()).toMatchObject({
      state: "error",
      readyUpdate: ready,
      error: { code: "install-folder-read-only" }
    })
    expect(value.wrapper.getState().restartRequired).toBeUndefined()
    blocked = false
    await value.wrapper.install(ready.id)
    expect(value.order).toEqual(["core stopped", "install"])
  })
  it.each([
    [
      "reports an error while installing",
      { quitAndInstall: () => {} },
      (value: ReturnType<typeof installable>) =>
        value.events.error(Error("Installer rejected"))
    ],
    [
      "throws",
      {
        quitAndInstall: () => {
          throw Error("Installer could not start")
        }
      },
      () => {}
    ]
  ] as const)(
    "says a restart is needed when the installer %s after the core stopped",
    async (_, options, fail) => {
      const value = installable(options as any)
      await value.wrapper.install().catch(() => {})
      fail(value)
      await settle()
      expect(value.order).toEqual(["core stopped", "install"])
      expect(value.wrapper.getState()).toMatchObject({
        state: "error",
        restartRequired: true
      })
    }
  )
  it("says a restart is needed when the update fails while the core stops", async () => {
    const value: ReturnType<typeof installable> = installable({
      shutdown: async () => {
        value.events.error(Error("Download replaced during shutdown"))
        return "stopped"
      }
    })
    await expect(value.wrapper.install()).rejects.toThrow("during shutdown")
    expect(value.order).toEqual(["core stopped"])
    expect(value.wrapper.getState()).toMatchObject({
      state: "error",
      restartRequired: true,
      error: { code: "update-failed" }
    })
  })
  it("does not ask for a restart when the player kept GDLauncher open", async () => {
    const value = installable({ shutdown: async () => "kept" })
    expect(await value.wrapper.install()).toBe("kept")
    value.events.error(Error("A later download failed"))
    expect(value.wrapper.getState().restartRequired).toBeUndefined()
  })
})
