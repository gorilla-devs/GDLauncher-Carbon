import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  pending: undefined as { pending: boolean; can_cancel: boolean } | undefined,
  queries: [] as { queryKey: unknown[]; enabled?: boolean }[],
  mutate: vi.fn(),
  close: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => (
    <button data-testid={p["data-testid"]} onClick={p.onClick}>
      {p.children}
    </button>
  )
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: () => ({ mutate: io.mutate }),
    createQuery: (
      options: () => { queryKey: unknown[]; enabled?: boolean }
    ) => {
      io.queries.push(options())
      return {
        get data() {
          return options().enabled ? io.pending : undefined
        }
      }
    }
  }
}))
vi.mock("../..", () => ({ useModal: () => ({ closeModal: io.close }) }))
vi.mock("../../ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))

import Confirmation from "./index"

let dispose: (() => void) | undefined
let root: HTMLElement
const open = (instanceState: "unlock" | "unpair") => {
  io.queries.length = 0
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <Confirmation
        title="Unpair Instance"
        data={{ instanceState, instanceId: 7 }}
      />
    ),
    root
  )
}
const note = () => root.querySelector('[data-testid="unpair-cancels-update"]')
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.clearAllMocks()
  io.pending = undefined
})

it("says before Unpair is confirmed that it cancels the pack's unfinished update, and only then", () => {
  io.pending = { pending: true, can_cancel: true }
  open("unpair")
  expect(io.queries[0]).toEqual({
    queryKey: ["instance.getPendingSetup", 7],
    enabled: true
  })
  expect(note()!.textContent).toBe(
    "general:_trn_instance_unpair_cancels_update"
  )
  root
    .querySelector<HTMLElement>('[data-testid="confirm-unpair-confirm"]')!
    .click()
  expect(io.mutate).toHaveBeenCalledWith({
    modpackLocked: { Set: null },
    instance: 7
  })
  expect(io.close).toHaveBeenCalled()
})

it("says nothing more when no update waits or when it cannot be cancelled", () => {
  open("unpair")
  expect(note()).toBeNull()
  dispose?.()
  io.pending = { pending: false, can_cancel: false }
  open("unpair")
  expect(note()).toBeNull()
  dispose?.()
  // One that replaced files: the launcher refuses the Unpair and says why.
  io.pending = { pending: true, can_cancel: false }
  open("unpair")
  expect(note()).toBeNull()
})

it("does not look for an update when unlocking", () => {
  io.pending = { pending: true, can_cancel: true }
  open("unlock")
  expect(io.queries[0].enabled).toBe(false)
  expect(note()).toBeNull()
})
