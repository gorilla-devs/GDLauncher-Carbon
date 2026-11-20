import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  open: vi.fn(),
  account: undefined as any
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => <button onClick={p.onClick}>{p.children}</button>,
  Progress: () => <div />,
  toast: { success: vi.fn(), error: vi.fn() }
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: { key: string }) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/utils/rspcClient", () => ({
  apiUrl: (path: string) => `/inert${path}`,
  rspc: {
    createQuery: () => ({
      isLoading: false,
      isError: false,
      data: {
        title: "Skyblock with friends",
        sharerFriendCode: "SOMEONE-ELSE",
        expiresAt: "2999-01-01T00:00:00Z",
        sizeKilobytes: 1024,
        mods: []
      }
    })
  }
}))
vi.mock("@/components/VirtuaWrapper", () => ({ VList: () => <div /> }))
vi.mock("/assets/images/icons/curseforge_logo.svg", () => ({ default: "" }))
vi.mock("/assets/images/icons/modrinth_logo.svg", () => ({ default: "" }))
vi.mock("/assets/images/default-instance-img.png", () => ({ default: "" }))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: io.open })
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    gdlAccount: {
      get data() {
        return io.account
      }
    }
  })
}))
vi.mock("@solidjs/router", () => ({
  useLocation: () => ({ pathname: "/search", search: "?share=abc" })
}))

import SharePreviewContent from "./SharePreviewContent"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  io.account = undefined
  vi.clearAllMocks()
})

function pressReport(account: unknown) {
  io.account = account
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <SharePreviewContent shareCode="abc" />, root)
  root
    .querySelector<HTMLElement>(
      'button[title="instances:_trn_share_preview.report_button"]'
    )!
    .click()
}

it.each(["unset", "skipped", "invalid"] as const)(
  "Report with a %s GDLauncher account asks to link one instead of opening a form that cannot be sent",
  (status) => {
    pressReport({ status })
    expect(io.open).toHaveBeenCalledTimes(1)
    expect(io.open).toHaveBeenCalledWith(
      { name: "requiresGdlAccount" },
      { returnPath: "/search?share=abc" }
    )
  }
)

it.each([
  ["a verified account", { status: "valid", value: { isEmailVerified: true } }],
  [
    "an account not verified yet",
    { status: "valid", value: { isEmailVerified: false } }
  ],
  ["an account the core has not answered about", undefined]
] as const)("Report with %s opens the report dialog", (_case, account) => {
  pressReport(account)
  expect(io.open).toHaveBeenCalledTimes(1)
  expect(io.open).toHaveBeenCalledWith(
    { name: "report" },
    {
      target: {
        kind: "share",
        shareCode: "abc",
        displayName: "Skyblock with friends"
      }
    }
  )
})
