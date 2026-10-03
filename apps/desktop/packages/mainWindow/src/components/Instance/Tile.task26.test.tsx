import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import type { ListInstance } from "@gd/core_module/bindings"
import Tile from "./Tile"

const c = vi.hoisted(() => ({ mutation: vi.fn() }))

vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/task26/ui")
  return {
    ...ui,
    ContextMenuPortal: ui.Wrapper,
    ContextMenuSub: ui.Wrapper,
    ContextMenuSubContent: ui.Wrapper,
    ContextMenuSubTrigger: ui.Wrapper
  }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (opts: any) => ({
      mutate: (v: any) => c.mutation(opts().mutationKey[0], v)
    })
  }
}))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "" }))
vi.mock("@/utils/instances", () => ({ getModpackPlatformIcon: () => "" }))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/managers/ModalsManager/modals/InstanceExport", () => ({
  setExportStep: vi.fn(),
  setPayload: vi.fn()
}))
vi.mock(
  "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent",
  () => ({ setCheckedFiles: vi.fn() })
)
vi.mock("@/components/InstanceTile", () => ({ setClickedInstanceId: vi.fn() }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({ currentlySelectedAccount: () => undefined })
}))
vi.mock("@/components/SearchInputContext", () => ({ default: () => undefined }))
vi.mock("@/components/GdlFeatureContextMenuItem", () => ({
  default: (p: any) => (
    <button disabled={p.disabled} onClick={() => p.onClick()}>
      {p.children}
    </button>
  )
}))
vi.mock("@/components/BaseTile", () => ({ BaseTile: () => <div /> }))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.innerHTML = ""
  c.mutation.mockReset()
})

const instance = (invalid: boolean): ListInstance =>
  ({
    id: 7,
    name: "Broken pack",
    favorite: true,
    status: invalid
      ? { status: "invalid", value: { type: "jsonMissing" } }
      : {
          status: "valid",
          value: {
            mc_version: "1.20.1",
            modloader: null,
            modloader_version: null,
            modpack: null,
            state: { state: "inactive", value: null }
          }
        }
  }) as unknown as ListInstance

const menu = (invalid: boolean) => {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <Tile
        instance={instance(invalid)}
        modloader={null}
        version={undefined}
        img={undefined}
        identifier="7"
        size={2}
        shouldSetViewTransition={false}
        isInvalid={invalid}
      />
    ),
    root
  )
  const item = (label: string) => {
    const found = [...root.querySelectorAll("button")].find((button) =>
      button.textContent?.includes(label)
    )
    expect(found, label).toBeDefined()
    return found!
  }
  return item
}

test("a tile whose instance.json is missing or broken offers only what still works", () => {
  const item = menu(true)
  for (const unusable of [
    "instances:_trn_action_play",
    "instances:_trn_action_edit",
    "instances:_trn_action_settings",
    "instances:_trn_instance_share.title",
    "instances:_trn_export_instance"
  ]) {
    expect(item(unusable).disabled, unusable).toBe(true)
  }
  for (const usable of [
    "instances:_trn_action_open_folder",
    "instances:_trn_remove_favorite",
    "instances:_trn_action_delete"
  ]) {
    expect(item(usable).disabled, usable).toBe(false)
  }
  item("instances:_trn_action_play").click()
  expect(c.mutation).not.toHaveBeenCalledWith(
    "instance.launchInstance",
    expect.anything()
  )
  item("instances:_trn_remove_favorite").click()
  expect(c.mutation).toHaveBeenCalledWith("instance.setFavorite", {
    instance: 7,
    favorite: false
  })
})

test("a valid tile still offers Play, Edit and Settings", () => {
  const item = menu(false)
  for (const usable of [
    "instances:_trn_action_play",
    "instances:_trn_action_edit",
    "instances:_trn_action_settings"
  ]) {
    expect(item(usable).disabled, usable).toBe(false)
  }
  item("instances:_trn_action_play").click()
  expect(c.mutation).toHaveBeenCalledWith("instance.launchInstance", {
    id: 7,
    skipMemoryCheck: false
  })
})
