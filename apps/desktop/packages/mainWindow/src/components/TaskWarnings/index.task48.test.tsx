import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { FETaskWarnings } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  data: (() => undefined) as () => FETaskWarnings[] | undefined,
  queryKey: null as unknown,
  acknowledge: vi.fn(),
  warning: vi.fn(),
  error: vi.fn(),
  openModal: vi.fn()
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { mutation: io.acknowledge } }),
    createQuery: (options: () => { queryKey: unknown }) => {
      io.queryKey = options().queryKey
      return {
        get data() {
          return io.data()
        }
      }
    }
  }
}))
vi.mock("@gd/ui", () => ({
  toast: { warning: io.warning, error: io.error }
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: io.openModal })
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) => {
      if (!options) return key
      const { interpolation: _, ...rest } = options
      return `${key} ${JSON.stringify(rest)}`
    }
  ]
}))

import TaskWarnings from "./index"

const entry = (id: number): FETaskWarnings => ({
  id,
  name: {
    translation: "InstanceTaskInstallMod",
    args: { mod_name: "Iris", instance_name: "Fabric 1.20.6" }
  },
  subject: null,
  warnings: [
    {
      type: "SkippedDependency",
      value: {
        name: "Sodium",
        platform: "Modrinth",
        project_id: "AANobbMI",
        reason: "NoCompatibleVersion"
      }
    }
  ]
})

let dispose: (() => void) | undefined
let setData: (value: FETaskWarnings[] | undefined) => void
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}

beforeEach(() => {
  vi.clearAllMocks()
  io.acknowledge.mockResolvedValue(null)
  const [data, set] = createSignal<FETaskWarnings[] | undefined>(undefined)
  io.data = data
  setData = (value) => set(() => value)
  dispose = render(() => <TaskWarnings />, document.createElement("div"))
})
afterEach(() => {
  dispose?.()
  dispose = undefined
})

it("reads the pending warnings and shows nothing while there are none", async () => {
  expect(io.queryKey).toEqual(["vtask.getTaskWarnings"])
  setData([])
  await flush()
  expect(io.warning).not.toHaveBeenCalled()
  expect(io.acknowledge).not.toHaveBeenCalled()
})

it("shows a task's warnings once, as a sticky toast, then acknowledges them", async () => {
  setData([entry(7)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(1)
  const [title, options] = io.warning.mock.calls[0]
  expect(title).toBe(
    'notifications:_trn_addon_dependencies_skipped_in_instance {"title":"Iris","instance":"Fabric 1.20.6"}'
  )
  expect(options.duration).toBe(Infinity)
  const description = options.description as HTMLElement
  expect(description.textContent).toContain(
    "notifications:_trn_addon_dependency_skipped_no_compatible_version"
  )
  expect(description.textContent).toContain(
    "notifications:_trn_addon_dependencies_skipped_hint"
  )
  expect(io.acknowledge).toHaveBeenCalledTimes(1)
  expect(io.acknowledge).toHaveBeenCalledWith([
    "vtask.acknowledgeTaskWarnings",
    7
  ])
  // The acknowledgement invalidation refetches the same entry before the
  // backend drops it: it is not shown again.
  setData([entry(7)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(1)
  expect(io.acknowledge).toHaveBeenCalledTimes(1)
  // A later task's warnings are shown on their own.
  setData([entry(7), entry(8)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(2)
  expect(io.acknowledge).toHaveBeenLastCalledWith([
    "vtask.acknowledgeTaskWarnings",
    8
  ])
})

it("does not toast again when the acknowledgement fails", async () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {})
  io.acknowledge.mockRejectedValueOnce(new Error("socket closed"))
  setData([entry(9)])
  await flush()
  expect(error).toHaveBeenCalledWith(
    "Could not acknowledge task warnings",
    expect.any(Error)
  )
  setData([entry(9)])
  await flush()
  expect(io.warning).toHaveBeenCalledTimes(1)
  error.mockRestore()
})

const lostFiles = (
  id: number,
  warning: FETaskWarnings["warnings"][number]
): FETaskWarnings => ({
  id,
  name: { translation: "InstanceTaskPrepare", args: { name: "ATM 10" } },
  subject: null,
  warnings: [warning]
})

it("offers Repair on an install that is missing files, and opens it for that instance", async () => {
  setData([
    lostFiles(11, {
      type: "PackFilesMissing",
      value: { instance: 5, files: ["mods/a.jar"], total: 1, repairable: true }
    })
  ])
  await flush()
  const [title, options] = io.warning.mock.calls[0]
  expect(title).toBe(
    'notifications:_trn_pack_files_missing_in_instance {"instance":"ATM 10","count":1}'
  )
  expect((options.description as HTMLElement).textContent).toBe(
    'notifications:_trn_pack_files_lost_single {"file":"mods/a.jar"}'
  )
  expect(options.action.label).toBe("instances:_trn_instance_settings.repair")
  options.action.onClick()
  expect(io.openModal).toHaveBeenCalledWith(
    { name: "repairModpack" },
    { id: 5, name: "ATM 10", isServer: false }
  )
  // Only the acknowledgement was sent to the core.
  expect(io.acknowledge).toHaveBeenCalledTimes(1)
})

it("shows no button when Repair cannot download the modpack again", async () => {
  setData([
    lostFiles(12, {
      type: "PackFilesMissing",
      value: { instance: 5, files: ["mods/a.jar"], total: 1, repairable: false }
    })
  ])
  await flush()
  expect(io.warning.mock.calls[0][1].action).toBeUndefined()
})

it("asks for the same version again from an update that was given up, and says why when that is refused", async () => {
  const modpack = {
    type: "curseforge",
    value: { project_id: 925200, file_id: 6000001 }
  } as const
  setData([
    lostFiles(13, {
      type: "PackUpdateAbandoned",
      value: { instance: 5, files: ["mods/a.jar"], total: 1, modpack }
    })
  ])
  await flush()
  const [title, options] = io.warning.mock.calls[0]
  expect(title).toBe(
    'notifications:_trn_pack_update_abandoned_in_instance {"instance":"ATM 10"}'
  )
  expect(options.action.label).toBe("notifications:_trn_pack_update_try_again")
  options.action.onClick()
  await flush()
  expect(io.acknowledge).toHaveBeenLastCalledWith([
    "instance.changeModpack",
    { instance: 5, modpack }
  ])
  expect(io.error).not.toHaveBeenCalled()

  io.acknowledge.mockRejectedValueOnce(new Error("the instance is running"))
  options.action.onClick()
  await flush()
  expect(io.error).toHaveBeenCalledTimes(1)
  expect(io.error.mock.calls[0][0]).toBe("instances:_trn_change_version_failed")
  expect(io.error.mock.calls[0][1].description).toContain(
    "the instance is running"
  )
})

it("opens the list of kept files from an update that kept some", async () => {
  const files = ["config/a.toml", "config/b.toml"]
  setData([
    lostFiles(14, { type: "PackFilesKept", value: { instance: 5, files } })
  ])
  await flush()
  const [title, options] = io.warning.mock.calls[0]
  expect(title).toBe(
    'notifications:_trn_modpack_updated_in_instance {"instance":"ATM 10"}'
  )
  expect((options.description as HTMLElement).textContent).toBe(
    'notifications:_trn_pack_files_kept {"count":2}'
  )
  expect(options.action.label).toBe("notifications:_trn_pack_files_kept_review")
  options.action.onClick()
  expect(io.openModal).toHaveBeenCalledWith(
    { name: "keptPackFiles" },
    { id: 5, name: "ATM 10", files }
  )
})
