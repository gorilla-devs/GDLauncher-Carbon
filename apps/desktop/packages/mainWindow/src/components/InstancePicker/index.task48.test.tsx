import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal, For, Show } from "solid-js"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  scrollToIndex: vi.fn(),
  popover: undefined as
    | { open: boolean; onOpenChange: (open: boolean) => void }
    | undefined
}))
vi.mock("@gd/ui", () => ({
  // One popover at a time: its parts find it here.
  Popover: (p: any) => {
    io.popover = p
    return <>{p.children}</>
  },
  PopoverTrigger: (p: any) => (
    <button
      data-testid="trigger"
      disabled={p.disabled}
      onClick={() => io.popover!.onOpenChange(!io.popover!.open)}
    >
      {p.children}
    </button>
  ),
  PopoverContent: (p: any) => (
    <Show when={io.popover!.open}>
      <div data-testid={p["data-testid"]}>{p.children}</div>
    </Show>
  ),
  Input: (p: any) => (
    <input
      data-testid={p["data-testid"]}
      placeholder={p.placeholder}
      value={p.value}
      aria-activedescendant={p["aria-activedescendant"]}
      onInput={p.onInput}
      onKeyDown={p.onKeyDown}
    />
  )
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key]
}))
// Draws every row: which rows are in view is the list's own business.
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => {
    p.ref?.({ scrollToIndex: io.scrollToIndex })
    return <For each={p.data}>{(item, index) => p.children(item, index)}</For>
  }
}))
vi.mock("@/utils/instances", () => ({
  getInstanceImageUrl: (id: number, revision: number) =>
    `icon/${id}/${revision}`
}))

import InstancePicker, {
  matching,
  moved,
  pickable,
  type PickableInstance
} from "./index"

const row = (
  id: number,
  name: string,
  gameVersion = "1.21",
  modloader = "vanilla"
): PickableInstance => ({ id, name, gameVersion, modloader, iconUrl: null })
const INSTANCES = [
  row(1, "Vanilla"),
  row(2, "Create with friends", "1.20.1", "forge"),
  row(3, "Skyblock", "1.20.1", "fabric"),
  row(4, "Old world", "1.12.2", "forge")
]

let dispose: (() => void) | undefined
let root: HTMLElement
const picked = vi.fn()
const part = (id: string) =>
  root.querySelector<HTMLElement>(`[data-testid="${id}"]`)
const names = () =>
  [
    ...root.querySelectorAll<HTMLElement>(
      '[data-testid="instance-picker-name"]'
    )
  ].map((shown) => shown.textContent)
const type = (text: string) => {
  const box = part("instance-picker-search") as HTMLInputElement
  box.value = text
  box.dispatchEvent(new InputEvent("input", { bubbles: true }))
}
const press = (key: string) =>
  part("instance-picker-search")!.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true })
  )
const lit = () =>
  root.querySelector(
    '[role="option"][aria-selected="true"] [data-testid="instance-picker-name"]'
  )?.textContent

beforeEach(() => {
  vi.clearAllMocks()
  root = document.createElement("div")
  document.body.append(root)
  const [instances] = createSignal(INSTANCES)
  dispose = render(
    () => (
      <InstancePicker
        instances={instances}
        onPick={picked}
        trigger={() => null}
        label="Test in"
      >
        Test
      </InstancePicker>
    ),
    root
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})

it("shows nothing until its button is pressed, then every instance under what picking does", () => {
  expect(part("instance-picker")).toBeNull()
  part("trigger")!.click()
  expect(part("instance-picker")!.textContent).toContain("Test in")
  expect(names()).toEqual([
    "Vanilla",
    "Create with friends",
    "Skyblock",
    "Old world"
  ])
  expect(
    root.querySelector('[data-testid="instance-picker-row"]')!.textContent
  ).toContain("1.21 • vanilla")
})

it("narrows the list by name, game version or loader as the box is typed in", () => {
  part("trigger")!.click()
  type("sky")
  expect(names()).toEqual(["Skyblock"])
  type("1.20")
  expect(names()).toEqual(["Create with friends", "Skyblock"])
  type("  FORGE ")
  expect(names()).toEqual(["Create with friends", "Old world"])
  type("nothing like it")
  expect(names()).toEqual([])
  expect(part("instance-picker-none")!.textContent).toBe(
    "content:_trn_common.no_instances_found"
  )
  // Enter on an empty list picks nothing.
  press("Enter")
  expect(picked).not.toHaveBeenCalled()
})

it("picks with the arrow keys and Enter from the box, keeping the lit row in view", () => {
  part("trigger")!.click()
  expect(lit()).toBe("Vanilla")
  press("ArrowUp")
  expect(lit()).toBe("Vanilla")
  press("ArrowDown")
  press("ArrowDown")
  expect(lit()).toBe("Skyblock")
  expect(io.scrollToIndex).toHaveBeenLastCalledWith(2, { align: "nearest" })
  press("PageDown")
  expect(lit()).toBe("Old world")
  // Typing starts again from the first match.
  type("forge")
  expect(lit()).toBe("Create with friends")
  press("ArrowDown")
  press("Enter")
  expect(picked).toHaveBeenCalledWith(4)
  expect(part("instance-picker")).toBeNull()
})

it("picks the row that is clicked, and opens empty of the last search", () => {
  part("trigger")!.click()
  type("sky")
  root
    .querySelector<HTMLElement>('[data-testid="instance-picker-row"]')!
    .click()
  expect(picked).toHaveBeenCalledWith(3)
  expect(part("instance-picker")).toBeNull()
  part("trigger")!.click()
  expect((part("instance-picker-search") as HTMLInputElement).value).toBe("")
  expect(names()).toHaveLength(4)
  expect(lit()).toBe("Vanilla")
})

it("matches, moves and maps on its own", () => {
  expect(matching(INSTANCES, "")).toBe(INSTANCES)
  expect(matching(INSTANCES, "1.12").map((found) => found.id)).toEqual([4])
  expect(moved(0, -1, 4)).toBe(0)
  expect(moved(3, 1, 4)).toBe(3)
  expect(moved(1, 6, 4)).toBe(3)
  expect(moved(0, 1, 0)).toBe(0)
  const listed = {
    id: 5,
    name: "Pack",
    icon_revision: 2,
    status: {
      status: "valid",
      value: { mc_version: "1.20.1", modloader: "fabric" }
    }
  } as any
  expect(pickable(listed)).toEqual({
    id: 5,
    name: "Pack",
    gameVersion: "1.20.1",
    modloader: "fabric",
    iconUrl: "icon/5/2"
  })
  expect(
    pickable({
      ...listed,
      icon_revision: null,
      status: {
        status: "valid",
        value: { mc_version: "1.21", modloader: null }
      }
    })
  ).toMatchObject({ modloader: "vanilla", iconUrl: null })
})
