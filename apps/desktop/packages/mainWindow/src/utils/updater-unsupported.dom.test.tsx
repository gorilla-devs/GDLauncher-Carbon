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
// Linux installs electron-updater cannot update (nixpkgs, AUR repackages,
// unpacked AppImages, self-built zips): its check resolves null, and the
// channel's metadata decides what the player is told.
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
const noAppUpdateConfig = async () => {
  throw Error("ENOENT: app-update.yml")
}
async function fixture(manual: () => Promise<any>) {
  vi.stubGlobal("__APP_VERSION__", "2.0.40")
  const controller = createUpdaterController({
    version: "2.0.40",
    artifact: () => null,
    manual,
    prepareShutdown: async () => {},
    isDownloaded: () => false,
    broadcast: (state: any) =>
      listener?.(null, { ...state, revision: state.revision + offset }),
    updater: {
      channel: null,
      allowPrerelease: false,
      allowDowngrade: false,
      on: () => {},
      checkForUpdates: async () => null,
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
    dispose = d
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
    shown: () => store.toasts().find((t) => t.id === "updater-checking") as any,
    toasts: () => store.toasts()
  }
}
it.each([
  ["has no update metadata", noAppUpdateConfig],
  [
    "has no download for the newer version",
    async () => ({ version: "2.0.41", files: [], downloadUrl: null })
  ]
])(
  "an automatic check on an install that %s shows nothing, at launch or later",
  async (_, manual) => {
    const f = await fixture(manual as () => Promise<any>)
    await f.automatic()
    await f.automatic()
    expect(f.controller.getState().state).toBe("unsupported")
    expect(f.toasts()).toEqual([])
  }
)
it("a check the player asked for explains a missing update source in a notice they can retry", async () => {
  const f = await fixture(noAppUpdateConfig)
  await f.manual()
  expect(f.shown()).toMatchObject({
    type: "info",
    title: "app:_trn_update_check_failed",
    description: "app:_trn_update_unavailable_check_failed_description",
    closeButton: true
  })
  expect(f.shown().action.label).toBe("general:_trn_retry")
})
it("a check the player asked for names the newer version it has no download for", async () => {
  const f = await fixture(async () => ({
    version: "2.0.41",
    files: [],
    downloadUrl: null
  }))
  await f.manual()
  expect(f.shown()).toMatchObject({
    type: "info",
    title: "app:_trn_update_version",
    description: "app:_trn_update_unavailable_no_download_description"
  })
  expect(f.shown().action).toBeUndefined()
})
it("an install that runs the latest version is up to date, quietly or when asked", async () => {
  const f = await fixture(async () => ({ noUpdate: true }))
  await f.automatic()
  expect(f.toasts()).toEqual([])
  await f.manual()
  expect(f.shown()).toMatchObject({
    type: "success",
    title: "app:_trn_update_up_to_date"
  })
})
