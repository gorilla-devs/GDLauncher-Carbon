import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  mutate: vi.fn(),
  close: vi.fn(),
  navigate: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => (
    <button data-testid={p["data-testid"]} onClick={p.onClick}>
      {p.children}
    </button>
  )
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: { key: string }) => <span>{p.key}</span>
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: { createMutation: () => ({ mutate: io.mutate }) }
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: io.navigate })
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    currentlySelectedAccount: () => ({
      username: "Steve",
      type: { type: "microsoft", value: { email: "steve@example.invalid" } }
    })
  })
}))
vi.mock("..", () => ({ useModal: () => ({ closeModal: io.close }) }))
vi.mock("../ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))

import AccountExpired from "./AccountExpired"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.clearAllMocks()
})

it("starts the game on the sign-in as it is, without going through the checks that led here again", () => {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <AccountExpired title="Account Expired" data={{ id: 7 }} />,
    root
  )
  root
    .querySelector<HTMLElement>(
      '[data-testid="account-expired-launch-anyway"]'
    )!
    .click()
  // Without skipAccountCheck Play would wait for a renewal again and ask
  // again; without skipMemoryCheck a memory question already answered could
  // come back and lead here a second time.
  expect(io.mutate).toHaveBeenCalledWith({
    id: 7,
    skipMemoryCheck: true,
    skipAccountCheck: true
  })
  expect(io.close).toHaveBeenCalled()
})
