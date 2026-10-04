import { afterEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import ModPackVersionUpdate from "."
import { deferred, settle } from "@/task33.fixture"

const c = vi.hoisted(() => ({
  change: vi.fn(),
  cancel: vi.fn(),
  close: vi.fn(),
  navigate: vi.fn(),
  installed: "a",
  versions: undefined as { id: string; name: string }[] | undefined,
  request: undefined as any
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({
  i18n: { t: (k: string) => k },
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [
    (k: string, o?: { platform?: string }) =>
      o?.platform ? `${k}(${o.platform})` : k
  ]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => (
    <div data-testid="layout" data-prevent-close={String(!!p.preventClose)}>
      {p.children}
    </div>
  )
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@/utils/rspcClient", async () => {
  const actual = await vi.importActual<any>("@/utils/rspcClient")
  return {
    ...actual,
    rspc: {
      createQuery: (f: any) => {
        const key = f().queryKey[0]
        if (key === "instance.getInstanceDetails") {
          return {
            data: {
              modpack: {
                modpack: {
                  type: "modrinth",
                  value: { project_id: "pack", version_id: c.installed }
                }
              }
            },
            isLoading: false
          }
        }
        if (key === "modplatforms.modrinth.getProjectVersions") {
          return { data: c.versions, isLoading: false }
        }
        return { data: undefined, isLoading: false }
      },
      createMutation: (f: any) => {
        if (f().mutationKey[0] === "instance.cancelModpackChangeCheck") {
          return { mutate: c.cancel }
        }
        const [pending, setPending] = createSignal(false)
        return {
          get isPending() {
            return pending()
          },
          mutateAsync: (input: any) => {
            c.change(input)
            setPending(true)
            return c.request.promise.finally(() => setPending(false))
          }
        }
      }
    }
  }
})

let dispose: undefined | (() => void)
let root: HTMLElement

// Wrapped, so a list that failed to load (no data) isn't taken for the
// default.
function open(
  installed: string,
  versions: { list: typeof c.versions } = {
    list: [
      { id: "a", name: "A" },
      { id: "b", name: "B" }
    ]
  }
) {
  c.installed = installed
  c.versions = versions.list
  c.request = deferred()
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <ModPackVersionUpdate title="" data={{ instanceId: 7 }} />,
    root
  )
}
const confirm = () =>
  root.querySelector<HTMLButtonElement>(
    '[data-testid="modpack-version-update-confirm"]'
  )!
const cancel = () =>
  root.querySelector<HTMLButtonElement>(
    '[data-testid="modpack-version-update-cancel"]'
  )!
const layout = () => root.querySelector('[data-testid="layout"]')!
const notListed = () =>
  root.querySelector('[data-testid="modpack-version-not-listed"]')
const noneListed = () =>
  root.querySelector('[data-testid="modpack-version-none-listed"]')
const failure = () =>
  root.querySelector('[data-testid="modpack-version-update-error"]')
const coded = (code: string, data?: unknown) =>
  new Error(
    JSON.stringify({
      code: 500,
      message: JSON.stringify({ cause: [{ display: "English", code, data }] })
    })
  )

afterEach(() => {
  dispose?.()
  dispose = undefined
  root.remove()
  vi.clearAllMocks()
  vi.useRealTimers()
})

it("won't change to an installed version the platform no longer lists", async () => {
  open("removed")
  await settle()
  expect(root.querySelector("select")!.dataset.value).toBeUndefined()
  expect(notListed()).not.toBeNull()
  expect(confirm().disabled).toBe(true)
  confirm().click()
  expect(c.change).not.toHaveBeenCalled()
  ;(root.querySelector("select") as any).choose("b")
  await settle()
  expect(notListed()).toBeNull()
  expect(confirm().disabled).toBe(false)
  confirm().click()
  expect(c.change).toHaveBeenCalledWith({
    instance: 7,
    modpack: {
      type: "modrinth",
      value: { project_id: "pack", version_id: "b" }
    }
  })
})

it("keeps the listed installed version selected", async () => {
  open("a")
  await settle()
  expect(root.querySelector("select")!.dataset.value).toBe("a")
  expect(notListed()).toBeNull()
  expect(noneListed()).toBeNull()
  expect(confirm().disabled).toBe(false)
})

// A list that came back empty, and one that failed to load (no data).
it.each([[[]], [undefined]])(
  "says why and what to do when no version can be listed (%j)",
  async (versions) => {
    open("a", { list: versions })
    await settle()
    expect(noneListed()!.textContent).toBe(
      "instances:_trn_change_version_none_listed"
    )
    expect(notListed()).toBeNull()
    expect(confirm().disabled).toBe(true)
    confirm().click()
    expect(c.change).not.toHaveBeenCalled()
  }
)

it("can't be dismissed during the check, and Cancel stops the check", async () => {
  open("a")
  await settle()
  expect(layout().getAttribute("data-prevent-close")).toBe("false")
  expect(root.querySelector("select")!.disabled).toBe(false)
  confirm().click()
  expect(layout().getAttribute("data-prevent-close")).toBe("true")
  expect(confirm().disabled).toBe(true)
  // What was clicked is what changes: the choice can't move during the check.
  expect(root.querySelector("select")!.disabled).toBe(true)

  cancel().click()
  expect(c.cancel).toHaveBeenCalledWith(7)
  expect(c.close).not.toHaveBeenCalled()
  expect(cancel().disabled).toBe(true)

  c.request.reject(coded("MODPACK_CHECK_CANCELLED"))
  await settle()
  expect(c.close).toHaveBeenCalledTimes(1)
  expect(c.navigate).not.toHaveBeenCalled()
  expect(failure()).toBeNull()
})

it("Cancel only closes the modal when nothing is running", async () => {
  open("a")
  await settle()
  cancel().click()
  expect(c.cancel).not.toHaveBeenCalled()
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("says the check is running only once it takes a while", async () => {
  vi.useFakeTimers()
  open("a")
  const checking = () =>
    root.querySelector('[data-testid="modpack-version-update-checking"]')
  confirm().click()
  expect(checking()).toBeNull()
  vi.advanceTimersByTime(500)
  expect(checking()).not.toBeNull()

  c.request.resolve(31)
  await vi.advanceTimersByTimeAsync(0)
  expect(c.close).toHaveBeenCalledTimes(1)
  expect(c.navigate).toHaveBeenCalledWith("/library")
})

it("words a coded refusal in the player's language and stays open", async () => {
  open("a")
  await settle()
  confirm().click()
  c.request.reject(coded("MODPACK_CHECK_UNREACHABLE", { platform: "Modrinth" }))
  await settle()
  expect(failure()!.textContent).toContain(
    "instances:_trn_change_version_unreachable(Modrinth)"
  )
  expect(failure()!.textContent).not.toContain("English")
  expect(root.querySelector("select")!.disabled).toBe(false)
  expect(c.close).not.toHaveBeenCalled()
  expect(c.navigate).not.toHaveBeenCalled()
  expect(layout().getAttribute("data-prevent-close")).toBe("false")
})

it("touches nothing when something else closed it before the end", async () => {
  open("a")
  await settle()
  confirm().click()
  dispose!()
  dispose = undefined
  c.request.resolve(31)
  await settle()
  expect(c.close).not.toHaveBeenCalled()
  expect(c.navigate).not.toHaveBeenCalled()
})
