import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
import { InstallButton } from "./InstallButton"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  document.body.replaceChildren()
})

it.each([
  [true, "content:_trn_datapack_downloaded"],
  [false, "content:_trn_mod.downloaded"]
])("an installed button with staged %s says %s", (staged, label) => {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <InstallButton
        loading={() => false}
        progress={() => null}
        isInstalled={() => true}
        instanceLocked={() => false}
        installedMod={() => undefined}
        onDownload={() => {}}
        staged={staged}
      />
    ),
    root
  )
  expect(root.textContent).toContain(label)
})
