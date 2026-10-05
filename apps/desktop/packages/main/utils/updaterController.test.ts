import { describe, expect, it } from "vitest"
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
    value.events["update-downloaded"]()
    await value.wrapper.install()
    expect(value.shutdowns).toBe(1)
    expect(value.installed).toBe(1)
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
