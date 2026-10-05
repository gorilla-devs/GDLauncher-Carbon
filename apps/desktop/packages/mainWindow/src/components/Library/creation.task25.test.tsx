import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"

const control = vi.hoisted(() => ({ open: vi.fn(), mutation: vi.fn() }))
vi.mock("@gd/ui", () => import("@/test/task25/ui"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    settings: { data: { reducedMotion: true } },
    minecraftVersions: { data: [{ id: "1.21.4", type: "release" }] }
  })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({
    openModal: control.open,
    closeModal: vi.fn(),
    hasOpenModals: () => false
  })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (props: any) => <div>{props.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { invalidateQueries: vi.fn() },
  rspc: {
    createQuery: () => ({ data: [] }),
    createMutation: (opts: any) => ({
      mutate: vi.fn(),
      mutateAsync: (payload: any) =>
        control.mutation(opts().mutationKey[0], payload),
      isPending: false
    })
  }
}))
vi.mock("@/pages/Library/DragContext", () => ({
  useDragContext: () =>
    new Proxy(
      {},
      { get: (_, key) => () => (key === "draggedIds" ? [] : false) }
    )
}))
vi.mock("@/components/InstanceTile", () => ({ default: () => <div /> }))
vi.mock("@/components/Server/Tile", () => ({ default: () => <div /> }))
vi.mock("@/pages/Library/components/DropPreviewTile", () => ({
  default: () => <div />
}))
vi.mock("@/pages/Library/components/EndOfGroupDropZone", () => ({
  EndOfGroupDropZone: () => <div />
}))
vi.mock("@/pages/Library/hooks/useDragLayoutAnimation", () => ({
  useDragLayoutAnimation: vi.fn()
}))
vi.mock("@/hooks/useDragSelect", () => ({
  useDragSelect: () => ({ selectionRect: () => null, handlers: {} })
}))
vi.mock("@/utils/adhelper", () => ({ default: { width: 0 } }))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: vi.fn(),
  getServerImageUrl: vi.fn()
}))
vi.mock("@/utils/sidebar", () => ({ getModloaderIcon: () => "" }))
import Expanded from "@task25-expanded"
import Creation from "@task25-creation"

let root: HTMLDivElement
let overlay: HTMLDivElement
let dispose: () => void
beforeEach(() => {
  vi.clearAllMocks()
  control.mutation.mockResolvedValue(undefined)
  root = document.createElement("div")
  overlay = document.createElement("div")
  overlay.id = "overlay"
  document.body.append(root, overlay)
})
afterEach(() => {
  dispose?.()
  root.remove()
  overlay.remove()
})
function mountFolder(kind: "instance" | "server") {
  dispose = render(
    () => (
      <Expanded
        group={{
          id: 2,
          name: "Synthetic folder",
          kind,
          instances: [],
          libraryPosition: 0
        }}
        isServerMode={kind === "server"}
        onClose={vi.fn()}
        tileSize={2}
        isDefaultGroup={false}
        selectedIds={new Set()}
        onToggleSelection={vi.fn()}
        onSetSelection={vi.fn()}
        onDragStart={vi.fn()}
      />
    ),
    root
  )
}
function find(key: string, where = root) {
  return [...where.querySelectorAll("button")].find((button) =>
    button.textContent?.includes(key)
  )!
}
test("server folder Create routes to serverCreation and composed submission uses its same-ID server group", async () => {
  mountFolder("server")
  const create =
    find("_trn_server_create", overlay) ??
    find("_trn_create_new_instance", overlay)
  create.click()
  expect(control.open).toHaveBeenCalledExactlyOnceWith(
    { name: "serverCreation" },
    { groupId: 2 }
  )
  const data = control.open.mock.calls[0][1]
  dispose()
  dispose = render(() => <Creation data={data} />, root)
  find("_trn_server_creation_create").click()
  await Promise.resolve()
  expect(control.mutation).toHaveBeenCalledWith(
    "server.createServer",
    expect.objectContaining({
      group: 2,
      gameVersion: "1.21.4",
      port: 25565,
      modloaderType: null
    })
  )
})
test("server folders withhold the instance import caller", () => {
  mountFolder("server")
  expect(find("_trn_import_instance", overlay)).toBeUndefined()
})
test("original/ordinary creation group payload remains explicit when supplied", async () => {
  dispose = render(() => <Creation data={{ groupId: 2 }} />, root)
  find("_trn_server_creation_create").click()
  await Promise.resolve()
  expect(control.mutation).toHaveBeenCalledWith(
    "server.createServer",
    expect.objectContaining({ group: 2 })
  )
})
test.each([undefined, -1, "2", 1.5])(
  "ordinary creation without a valid supplied group (%s) retains null",
  async (groupId) => {
    dispose = render(() => <Creation data={{ groupId }} />, root)
    find("_trn_server_creation_create").click()
    await Promise.resolve()
    expect(control.mutation).toHaveBeenCalledWith(
      "server.createServer",
      expect.objectContaining({ group: null })
    )
  }
)
test.each([false, true])(
  "client folder preserves its Create/import(%s) namespace and group payload",
  (importing) => {
    mountFolder("instance")
    find(
      importing ? "_trn_import_instance" : "_trn_create_new_instance",
      overlay
    ).click()
    expect(control.open).toHaveBeenCalledExactlyOnceWith(
      { name: "instanceCreation" },
      { groupId: 2, ...(importing ? { import: true } : {}) }
    )
  }
)
