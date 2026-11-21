import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import type { ListInstance } from "@gd/core_module/bindings"
import Tile from "./Tile"

const c = vi.hoisted(() => ({
  mutation: vi.fn(),
  // What the core answers for the instance that has the install turn.
  turn: undefined as { instance: number; name: string } | undefined,
  turnAsked: false,
  tile: undefined as any
}))

vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/uiMenus.fixture")
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
    }),
    createQuery: (opts: any) => ({
      get data() {
        expect(opts().queryKey).toEqual(["instance.getInstallTurn"])
        c.turnAsked = opts().enabled
        return opts().enabled ? c.turn : undefined
      }
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
vi.mock("@/components/BaseTile", () => ({
  BaseTile: (p: any) => {
    c.tile = p
    return <div />
  }
}))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.innerHTML = ""
  c.mutation.mockReset()
  c.turn = undefined
  c.turnAsked = false
  c.tile = undefined
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

const menu = (invalid: boolean, queued = false) => {
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
        isQueued={queued}
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

test("a queued tile says what it waits for and can leave the queue", () => {
  c.turn = { instance: 3, name: "All the Mods 10" }
  const item = menu(false, true)
  expect(c.tile.isWaiting).toBe(true)
  expect(c.tile.waitingDetail).toBe("instances:_trn_queue_after")
  expect(c.turnAsked).toBe(true)
  expect(c.tile.leaveWaitingLabel).toBe("instances:_trn_leave_queue")
  c.tile.onLeaveWaiting()
  expect(c.mutation).toHaveBeenLastCalledWith("instance.leaveQueue", 7)

  // Play makes no sense while it waits; the menu offers the way out instead.
  expect(
    [...document.querySelectorAll("button")].some((button) =>
      button.textContent?.includes("instances:_trn_action_play")
    )
  ).toBe(false)
  c.mutation.mockReset()
  item("instances:_trn_leave_queue").click()
  expect(c.mutation).toHaveBeenCalledWith("instance.leaveQueue", 7)
})

test("a tile that is not queued asks nothing about the queue", () => {
  c.turn = { instance: 3, name: "All the Mods 10" }
  menu(false)
  expect(c.tile.isWaiting).toBe(false)
  expect(c.tile.waitingDetail).toBeFalsy()
  expect(c.turnAsked).toBe(false)
})
