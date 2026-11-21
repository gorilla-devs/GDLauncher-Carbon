import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  navigate: vi.fn(),
  close: vi.fn(),
  closeAll: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => <button onClick={p.onClick}>{p.children}</button>
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: { key: string }) => <span>{p.key}</span>
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: io.navigate })
}))
vi.mock("..", () => ({
  useModal: () => ({ closeModal: io.close, closeAllModals: io.closeAll })
}))
vi.mock("../ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))

import RequiresGdlAccountModal from "./RequiresGdlAccountModal"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.clearAllMocks()
})

function press(key: string, returnPath = "/search?share=abc") {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <RequiresGdlAccountModal
        title="Requires GDLauncher Account"
        data={{ returnPath }}
      />
    ),
    root
  )
  ;[...root.querySelectorAll("button")]
    .find((b) => b.textContent?.includes(key))!
    .click()
}

it("Link Account leaves no dialog open over the sign-in page", () => {
  press("_trn_link_account")
  expect(io.navigate).toHaveBeenCalledWith(
    "/?addGdlAccount=true&returnTo=%2Fsearch%3Fshare%3Dabc"
  )
  expect(io.closeAll).toHaveBeenCalledTimes(1)
})

it.each([
  ["/library?m[1]=sharePreview&m[2]=requiresGdlAccount", "%2Flibrary"],
  ["/library?m%5B1%5D=sharePreview&tab=mods", "%2Flibrary%3Ftab%3Dmods"]
])(
  "Link Account does not return to %s with dialogs named that are closed by then",
  (returnPath, encoded) => {
    press("_trn_link_account", returnPath)
    expect(io.navigate).toHaveBeenCalledWith(
      `/?addGdlAccount=true&returnTo=${encoded}`
    )
  }
)

it("Cancel closes only this dialog", () => {
  press("_trn_cancel")
  expect(io.close).toHaveBeenCalledTimes(1)
  expect(io.closeAll).not.toHaveBeenCalled()
  expect(io.navigate).not.toHaveBeenCalled()
})
