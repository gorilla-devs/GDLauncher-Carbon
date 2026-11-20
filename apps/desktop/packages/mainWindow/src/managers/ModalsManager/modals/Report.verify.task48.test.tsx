import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  account: undefined as any,
  report: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => <button onClick={p.onClick}>{p.children}</button>,
  Select: () => <div />,
  SelectContent: () => <div />,
  SelectItem: () => <div />,
  SelectTrigger: () => <div />,
  SelectValue: () => <div />,
  TextArea: () => <textarea />,
  toast: { success: vi.fn(), error: vi.fn() }
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: { key: string }) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: () => ({ mutateAsync: io.report, isPending: false })
  }
}))
vi.mock("@/components/SharePreviewContent", () => ({
  getErrorCode: () => null
}))
vi.mock("@/components/VerificationRequiredPlaceholder", () => ({
  default: () => <div data-testid="verification-required" />
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
vi.mock("..", () => ({ useModal: () => ({ closeModal: vi.fn() }) }))
vi.mock("../ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))

import Report from "./Report"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  io.account = undefined
  vi.clearAllMocks()
})

function open(account: unknown) {
  io.account = account
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <Report
        title="Report content"
        data={{ target: { kind: "share", shareCode: "abc" } }}
      />
    ),
    root
  )
  return {
    verification: root.querySelector('[data-testid="verification-required"]'),
    submit: [...root.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("_trn_report.submit")
    )
  }
}

it("shows the verification screen in place of a form the server would refuse", () => {
  const dialog = open({ status: "valid", value: { isEmailVerified: false } })
  expect(dialog.verification).not.toBeNull()
  expect(dialog.submit).toBeUndefined()
})

it.each([
  ["a verified account", { status: "valid", value: { isEmailVerified: true } }],
  ["an account the core has not answered about", undefined]
] as const)("shows the form for %s", async (_case, account) => {
  const dialog = open(account)
  expect(dialog.verification).toBeNull()
  io.report.mockResolvedValue(undefined)
  dialog.submit!.click()
  await Promise.resolve()
  expect(io.report).toHaveBeenCalledWith({
    shareCode: "abc",
    reportType: "share_background",
    reason: null
  })
})
