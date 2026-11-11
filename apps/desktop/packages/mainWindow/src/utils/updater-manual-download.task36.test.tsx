import { afterEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { render } from "solid-js/web"
import { toast, Toaster, useSomoto } from "somoto"
import { createUpdaterController } from "../../../main/utils/updaterController"
vi.mock("@gd/ui", async () => ({
  toast: (await import("somoto")).toast,
  Progress: () => <span />
}))
vi.mock("@gd/i18n", () => ({
  i18n: {
    language: "english",
    on: vi.fn(),
    t: (key: string, options?: { version?: string }) =>
      options?.version ? `${key}:${options.version}` : key
  }
}))
vi.mock("./rspcClient", () => ({
  rspc: {
    createQuery: () => {
      throw Error("No transport/query admitted")
    }
  }
}))
// Installs that cannot update themselves but whose channel lists a download
// for them get a notice with a Copy Link button: electron-updater is inactive
// (a Linux zip, an unpacked AppImage), or it downloads a deb or rpm package
// that only the system package manager can install.
const NOTICE = "updater-manual-update"
const ZIP = "https://cdn.invalid/GDLauncher__2.0.41__linux__x64.zip"
const DEB = "https://cdn.invalid/GDLauncher__2.0.41__linux__amd64.deb"
const newer = async () => ({ version: "2.0.41", files: [], downloadUrl: ZIP })
let renderer: typeof import("./updater") | undefined,
  listener: ((_: unknown, state: any) => void) | undefined
const disposers: (() => void)[] = []
let offset = 0
const flush = async () => {
  for (let i = 0; i < 40; i++) await Promise.resolve()
}
afterEach(() => {
  // The module keeps its state between tests: a finished check that found
  // nothing new closes the notice and forgets that it was shown.
  listener?.(null, {
    revision: offset + 999,
    state: "no-update",
    updateInfo: null,
    progress: 0,
    error: null
  })
  disposers.splice(0).forEach((d) => d())
  toast.dismiss()
  offset += 1000
  vi.useRealTimers()
  vi.unstubAllGlobals()
  document.body.textContent = ""
})
const titleOf = (shown: any) => {
  let title = shown?.title
  while (typeof title === "function") title = title()
  return String(title)
}
async function fixture(updater: {
  manual?: () => Promise<any>
  on?: (event: string, callback: (...args: any[]) => void) => void
  checkForUpdates?: () => Promise<unknown>
  artifact?: () => string | null
}) {
  vi.stubGlobal("__APP_VERSION__", "2.0.40")
  const controller = createUpdaterController({
    version: "2.0.40",
    artifact: updater.artifact ?? (() => null),
    manual: updater.manual ?? (async () => null),
    prepareShutdown: async () => {},
    isDownloaded: () => true,
    broadcast: (state: any) =>
      listener?.(null, { ...state, revision: state.revision + offset }),
    updater: {
      channel: null,
      allowPrerelease: false,
      allowDowngrade: false,
      on: updater.on ?? (() => {}),
      checkForUpdates: updater.checkForUpdates ?? (async () => null),
      quitAndInstall: () => {}
    }
  })
  Object.assign(window, {
    getCurrentOS: async () => ({ supportsAutoUpdate: false }),
    getUpdateState: async () => ({
      ...controller.getState(),
      revision: controller.getState().revision + offset
    }),
    onUpdateStateChanged: (cb: any) => {
      listener = cb
    },
    checkForUpdates: (channel: any) => controller.check(channel)
  })
  renderer ??= await import("./updater")
  let store!: ReturnType<typeof useSomoto>
  createRoot((d) => {
    disposers.push(d)
    store = useSomoto()
  })
  await flush()
  return {
    controller,
    // What the launch check and the 30-minute timer do.
    automatic: async () => {
      await window.checkForUpdates("stable")
      await flush()
    },
    manual: async () => {
      await renderer!.manualCheckForUpdates("stable")
      await flush()
    },
    find: (id: string) => store.toasts().find((t) => t.id === id) as any
  }
}
// electron-updater is inactive: its check resolves null and the channel's
// metadata names the newer version and its zip.
async function inactiveUpdater() {
  let answer: () => Promise<any> = newer
  const f = await fixture({ manual: () => answer() })
  return {
    ...f,
    answer: (next: () => Promise<any>) => {
      answer = next
    },
    notice: () => f.find(NOTICE)
  }
}

it("an install that cannot update itself keeps the manual download notice up through later automatic checks", async () => {
  const f = await inactiveUpdater()
  await f.automatic()
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
  await f.automatic()
  expect(f.controller.getState().state).toBe("manual-update")
  expect(f.notice()).toBeDefined()
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
})

it("a check the player asks for answers with the manual download notice and clears Checking", async () => {
  const f = await inactiveUpdater()
  await f.automatic()
  await f.manual()
  expect(f.find("updater-checking")).toBeUndefined()
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
  let copied = ""
  vi.stubGlobal("navigator", {
    clipboard: {
      writeText: async (text: string) => {
        copied = text
      }
    }
  })
  await f.notice().action.onClick()
  expect(copied).toBe(ZIP)
})

it("after the player closed the notice, automatic checks stay quiet and a check the player asks for shows it again", async () => {
  const f = await inactiveUpdater()
  await f.automatic()
  expect(f.notice()).toBeDefined()
  toast.dismiss(NOTICE)
  await flush()
  await f.automatic()
  await f.automatic()
  expect(f.notice()).toBeUndefined()
  await f.manual()
  expect(f.find("updater-checking")).toBeUndefined()
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
})

it("the kept notice still names its version while a later check runs", async () => {
  const f = await inactiveUpdater()
  await f.automatic()
  let release!: () => void
  f.answer(
    () =>
      new Promise((resolve) => {
        release = () => resolve(newer())
      })
  )
  const check = window.checkForUpdates("stable")
  await flush()
  expect(f.controller.getState().state).toBe("checking")
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
  release()
  await check
  await flush()
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
})

it("a later check that finds the install up to date closes the notice", async () => {
  const f = await inactiveUpdater()
  await f.automatic()
  expect(f.notice()).toBeDefined()
  f.answer(async () => ({ noUpdate: true }))
  await f.automatic()
  expect(f.notice()).toBeUndefined()
})

it("a later check that cannot reach the feed closes the notice and the next one offers it again", async () => {
  const f = await inactiveUpdater()
  await f.automatic()
  expect(f.notice()).toBeDefined()
  f.answer(async () => {
    throw Error("getaddrinfo ENOTFOUND cdn.invalid")
  })
  await f.automatic()
  expect(f.controller.getState().state).toBe("unsupported")
  expect(f.notice()).toBeUndefined()
  f.answer(newer)
  await f.automatic()
  expect(titleOf(f.notice())).toBe("app:_trn_update_version:2.0.41")
})

// A deb or rpm install: electron-updater downloads the package, which the
// launcher cannot install, so each check offers its link instead. somoto
// removes a closed toast 200 ms later, taking along a toast shown again under
// the same id within that time.
it.each([20, 400])(
  "on a deb or rpm install the notice survives a periodic check whose feed answers in %i ms",
  async (answersIn) => {
    vi.useFakeTimers()
    const handlers: Record<string, (...args: any[]) => void> = {}
    let feed = 300
    const f = await fixture({
      artifact: () => DEB,
      on: (event, callback) => {
        handlers[event] = callback
      },
      checkForUpdates: async () => {
        await new Promise((resolve) => setTimeout(resolve, feed))
        const updateInfo = {
          version: "2.0.41",
          path: "GDLauncher__2.0.41__linux__amd64.deb",
          sha512: "deb",
          releaseDate: "2026-11-01",
          files: [
            { url: "GDLauncher__2.0.41__linux__amd64.deb", sha512: "deb" }
          ]
        }
        handlers["update-available"](updateInfo)
        setTimeout(() => handlers["update-downloaded"](), 50)
        return { updateInfo, downloadPromise: Promise.resolve() }
      }
    })
    const container = document.createElement("div")
    document.body.append(container)
    disposers.push(render(() => <Toaster />, container))
    const visible = () =>
      [...container.querySelectorAll("[data-somoto-toast]")]
        .filter((node) => node.getAttribute("data-removed") !== "true")
        .map((node) => node.textContent)
        .join("\n")
    const periodic = async () => {
      const check = window.checkForUpdates("stable")
      await vi.advanceTimersByTimeAsync(600)
      await check
      await flush()
    }
    await periodic()
    expect(f.controller.getState().state).toBe("downloaded")
    expect(visible()).toContain("app:_trn_update_version:2.0.41")
    feed = answersIn
    await periodic()
    expect(f.controller.getState().state).toBe("downloaded")
    expect(visible()).toContain("app:_trn_update_version:2.0.41")
  }
)
