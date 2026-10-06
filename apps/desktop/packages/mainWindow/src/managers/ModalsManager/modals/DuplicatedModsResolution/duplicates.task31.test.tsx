import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import { button, settle } from "@/task31.fixture"
import Instance from "@/pages/Library/Instance"
import DuplicatedModsResolution from "."
import { detectDuplicatedMods } from "@/utils/duplicateMods"

const c = vi.hoisted(() => ({
  mods: [] as any[],
  open: vi.fn(),
  close: vi.fn(),
  mutate: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task31.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@solidjs/router", () => ({
  useParams: () => ({ id: "7" }),
  useLocation: () => ({ pathname: "/library/7/addons", search: "" })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: c.open, closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    markInstanceAsSeen: vi.fn(),
    gdlAccount: { data: null }
  })
}))
vi.mock("@/pages/Library/Instance/instance.data", () => ({
  default: () => ({
    instanceMods: c.mods,
    instanceDetails: {
      data: {
        id: 7,
        name: "Synthetic instance",
        modloaders: [],
        state: { type: "Inactive" }
      }
    },
    instancesUngrouped: { data: [{ id: 7 }], isFetching: false }
  })
}))
vi.mock("@/pages/Library/shared/DetailPageLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/pages/Library/Instance/Tabs/Log", () => ({
  isFullScreen: () => false
}))
vi.mock("@/pages/Library/Instance/Info/Authors", () => ({
  default: () => null
}))
vi.mock("@/components/FeatureStatusBadge", () => ({ default: () => null }))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "" }))
vi.mock("@/utils/helpers", () => ({ convertSecondsToHumanTime: () => "" }))
vi.mock("@/utils/instances", () => ({
  getModImageUrl: (_i: string, id: string) => `image:${id}`,
  getInstanceImageUrl: () => "",
  getRunningState: () => undefined,
  getPreparingState: () => undefined,
  getQueuedState: () => undefined
}))
vi.mock("@/managers/ModalsManager/modals/InstanceExport", () => ({
  setPayload: vi.fn(),
  setExportStep: vi.fn()
}))
vi.mock(
  "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent",
  () => ({ setCheckedFiles: vi.fn() })
)
vi.mock("@/utils/rspcClient", () => ({
  queryClient: {},
  rspc: {
    createQuery: () => ({ data: undefined }),
    createMutation: (f: any) => ({
      mutateAsync: (v: any) => c.mutate(f().mutationKey[0], v),
      mutate: vi.fn()
    })
  }
}))
let dispose: (() => void) | undefined
beforeEach(() => {
  vi.clearAllMocks()
  c.mutate.mockResolvedValue(undefined)
  c.mods = [
    mod("a1", "groupA"),
    mod("a2", "groupA"),
    mod("b1", "groupB"),
    mod("b2", "groupB")
  ]
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mod(id: string, group: string, name = "Example", enabled = true): any {
  return {
    id,
    filename: `${id}.jar`,
    enabled,
    file_size: 100,
    curseforge: { has_image: true },
    metadata: { modid: group, name, version: id, has_image: true }
  }
}
function mount(data?: any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => {
    const [opened, setOpened] = createSignal(data)
    c.open.mockImplementation((_m, d) => setOpened(d))
    return (
      <>
        {data ? null : <Instance />}
        {opened() && (
          <DuplicatedModsResolution
            title="Synthetic duplicates"
            data={opened()}
          />
        )}
      </>
    )
  }, root)
  if (!data) button(root, "fix_now").click()
  return root
}
function summary(root: HTMLElement, action = "disable") {
  button(root, "steps.action").click()
  root
    .querySelector<HTMLInputElement>(`input[value="${action}"]`)!
    .dispatchEvent(new Event("change", { bubbles: true }))
  button(root, "action.button_continue").click()
}
it("0035 detector→actual Instance caller carries distinct outer keys and separate image file IDs", () => {
  const root = mount()
  const data = c.open.mock.calls[0][1]
  expect(
    data.duplicatedMods.map((g: any) => ({
      groupKey: g.groupKey,
      modId: g.modId
    }))
  ).toEqual([
    { groupKey: "groupA", modId: "a1" },
    { groupKey: "groupB", modId: "b1" }
  ])
  summary(root)
  expect(root.querySelectorAll('img[src="image:a1"]')).toHaveLength(1)
  expect(root.querySelectorAll('img[src="image:b1"]')).toHaveLength(1)
})
it.each(["disable", "remove"])(
  "0035 equal names through real detector/caller/wizard retain exactly one in each group for %s",
  async (action) => {
    const root = mount()
    summary(root, action)
    button(root, "summary.button_apply").click()
    await settle()
    expect(c.mutate.mock.calls).toEqual([
      [
        `instance.${action === "remove" ? "delete" : "disable"}Mod`,
        { instance_id: 7, mod_id: "a2" }
      ],
      [
        `instance.${action === "remove" ? "delete" : "disable"}Mod`,
        { instance_id: 7, mod_id: "b2" }
      ]
    ])
    expect(c.close).toHaveBeenCalledTimes(1)
  }
)
it("0035 selections for equal names are independent and summary agrees with deletion plan", async () => {
  const root = mount()
  button(root, "steps.select_mods").click()
  root
    .querySelector<HTMLInputElement>('input[value="a2"]')!
    .dispatchEvent(new Event("change", { bubbles: true }))
  button(root, "selection.button_next").click()
  root
    .querySelector<HTMLInputElement>('input[value="b1"]')!
    .dispatchEvent(new Event("change", { bubbles: true }))
  button(root, "selection.button_continue").click()
  button(root, "action.button_continue").click()
  const kept = Array.from(root.querySelectorAll(".bg-darkSlate-800")).map(
    (e) => e.textContent
  )
  expect(kept.some((s) => s?.includes("a2.jar"))).toBe(true)
  expect(kept.some((s) => s?.includes("b1.jar"))).toBe(true)
  button(root, "summary.button_apply").click()
  await settle()
  expect(c.mutate.mock.calls.map(([, v]) => v.mod_id)).toEqual(["a1", "b2"])
})
it("0035 distinct-name control, disabled and singleton groups remain excluded", async () => {
  c.mods = [
    mod("a1", "A", "First"),
    mod("a2", "A", "First"),
    mod("b1", "B", "Second"),
    mod("b2", "B", "Second"),
    mod("disabled", "A", "First", false),
    mod("single", "S")
  ]
  expect(detectDuplicatedMods(c.mods).map((g) => g.modId)).toEqual(["A", "B"])
  const root = mount()
  summary(root)
  button(root, "summary.button_apply").click()
  await settle()
  expect(c.mutate.mock.calls.map(([, v]) => v.mod_id)).toEqual(["a2", "b2"])
})
it.each(["missingKey", "repeatedKey", "emptyGroup", "repeatedSelectedMember"])(
  "0035 invalid %s plan refuses all mutations",
  async (kind) => {
    const groups: any[] = detectDuplicatedMods(c.mods).map((g) => ({
      ...g.mods,
      groupKey: g.modId
    }))
    if (kind === "missingKey") delete groups[0].groupKey
    if (kind === "repeatedKey") groups[1].groupKey = groups[0].groupKey
    if (kind === "emptyGroup") groups[1].versions = []
    if (kind === "repeatedSelectedMember")
      groups[1].versions.push({ ...groups[1].versions[0] })
    const root = mount({ instanceId: 7, duplicatedMods: groups })
    summary(root)
    button(root, "summary.button_apply").click()
    await settle()
    expect(c.mutate).not.toHaveBeenCalled()
    expect(c.close).not.toHaveBeenCalled()
  }
)
