import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { Mod } from "@gd/core_module/bindings"
import { DatapackStaging } from "./DatapackStaging"

// The panel runs on the real @gd/ui (Kobalte) Selects; only the transport,
// toast and translations are adapted.
const c = vi.hoisted(() => ({
  copy: vi.fn(),
  worlds: [] as { identity: string; name: string }[]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({
      get data() {
        return c.worlds
      },
      isLoading: false,
      refetch: vi.fn()
    }),
    createMutation: () => ({
      mutateAsync: c.copy,
      isPending: false
    })
  }
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key],
  Trans: (p: any) => <span>{p.key}</span>
}))
vi.mock("@gd/ui", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@gd/ui")>()),
  toast: { success: vi.fn(), error: vi.fn() }
}))

const pack = (id: string, filename: string) => ({ id, filename }) as Mod
const packs = [
  pack("zip-pack", "Terralith.zip"),
  pack("folder-pack", "unpacked-folder")
]
let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  c.copy.mockReset()
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
  await new Promise((resolve) => setTimeout(resolve, 0))
}
function mount() {
  c.worlds = [
    { identity: "world-a", name: "Survival" },
    { identity: "world-b", name: "Creative" }
  ]
  const [instanceId, setInstanceId] = createSignal(7)
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <DatapackStaging instanceId={instanceId()} packs={packs} locked={false} />
    ),
    root
  )
  const trigger = (label: string) =>
    root.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!
  const addButton = () =>
    [...root.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("_trn_datapack_staging.add_to_world")
    )!
  return { root, trigger, addButton, setInstanceId }
}
const press = (element: Element, key: string) =>
  element.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })
  )
/** Opens a Select from its trigger and returns the offered option labels. */
async function open(trigger: HTMLElement) {
  press(trigger, "Enter")
  await flush()
  return [...document.querySelectorAll('[role="option"]')].map(
    (option) => option.textContent
  )
}
async function choose(trigger: HTMLElement, label: string) {
  await open(trigger)
  const option = [...document.querySelectorAll('[role="option"]')].find(
    (option) => option.textContent === label
  )!
  press(option, "Enter")
  await flush()
}

it("offers only ZIP datapacks and copies the chosen pack into the chosen world", async () => {
  c.copy.mockResolvedValue("Survival")
  const { trigger, addButton } = mount()
  await flush()
  expect(addButton().disabled).toBe(true)

  expect(
    await open(trigger("content:_trn_datapack_staging.pack_label"))
  ).toEqual(["Terralith.zip"])
  press(document.querySelector('[role="option"]')!, "Escape")
  await flush()

  await choose(
    trigger("content:_trn_datapack_staging.pack_label"),
    "Terralith.zip"
  )
  expect(
    trigger("content:_trn_datapack_staging.pack_label").textContent
  ).toContain("Terralith.zip")
  expect(addButton().disabled).toBe(true)

  await choose(trigger("content:_trn_datapack_staging.world_label"), "Creative")
  expect(
    trigger("content:_trn_datapack_staging.world_label").textContent
  ).toContain("Creative")
  expect(addButton().disabled).toBe(false)

  addButton().click()
  await flush()
  expect(c.copy).toHaveBeenCalledWith({
    instance_id: 7,
    mod_id: "zip-pack",
    world: { identity: "world-b", name: "Creative" }
  })
})

it("clears both choices when the panel moves to another instance", async () => {
  const { trigger, addButton, setInstanceId } = mount()
  await flush()
  await choose(
    trigger("content:_trn_datapack_staging.pack_label"),
    "Terralith.zip"
  )
  await choose(trigger("content:_trn_datapack_staging.world_label"), "Survival")
  expect(addButton().disabled).toBe(false)

  setInstanceId(8)
  await flush()
  expect(addButton().disabled).toBe(true)
  expect(
    trigger("content:_trn_datapack_staging.pack_label").textContent
  ).toContain("content:_trn_datapack_staging.choose_pack")
  expect(
    trigger("content:_trn_datapack_staging.world_label").textContent
  ).toContain("content:_trn_datapack_staging.choose_world")
})
