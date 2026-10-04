import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { QueryClientProvider } from "@tanstack/solid-query"
import { toast } from "@/task33.fixture"
import { queryClient } from "@/utils/rspcClient"
import ConfirmInstanceDeletion from "./ConfirmInstanceDeletion"

// The real rspc client and query client, so the global error handler runs
// with the dialog's own mutation options; only the core's reply is faked.
const c = vi.hoisted(() => ({ close: vi.fn(), navigate: vi.fn() }))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({
  i18n: { t: (k: string) => k },
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))

let dispose: (() => void) | undefined
const fetchMock = vi.fn()
beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(console, "error").mockImplementation(() => {})
  vi.stubGlobal("fetch", fetchMock)
  // The core refuses: rspc's error body, whose message is the core's error
  // with its debug text and backtrace.
  fetchMock.mockResolvedValue({
    ok: true,
    json: async () => ({
      result: {
        type: "error",
        data: {
          code: 500,
          message: JSON.stringify(
            {
              cause: [{ display: "It is still running", debug: "Debug" }],
              backtrace: "0: carbon_app::error::make_rspc"
            },
            null,
            2
          )
        }
      }
    })
  })
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  queryClient.clear()
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  document.body.textContent = ""
})

it.each([
  {
    isServer: false,
    procedure: "instance.deleteInstance",
    title: "notifications:_trn_cannot_delete_instance"
  },
  {
    isServer: true,
    procedure: "server.deleteServer",
    title: "notifications:_trn_cannot_delete_server"
  }
])(
  "a refused $procedure shows one toast with the core's sentence",
  async (step) => {
    const root = document.createElement("div")
    document.body.append(root)
    dispose = render(
      () => (
        <QueryClientProvider client={queryClient}>
          <ConfirmInstanceDeletion
            title="Delete"
            data={{ id: 7, name: "Synthetic", isServer: step.isServer }}
          />
        </QueryClientProvider>
      ),
      root
    )
    root
      .querySelector<HTMLButtonElement>(
        '[data-testid="confirm-instance-deletion"]'
      )!
      .click()
    await vi.waitFor(() => expect(toast.error).toHaveBeenCalled())
    // Let the global handler and any later toast run too.
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(String(fetchMock.mock.calls[0][0])).toMatch(
      new RegExp(`/rspc/${step.procedure}$`)
    )
    expect(toast.error).toHaveBeenCalledTimes(1)
    const [title, options] = (toast.error as any).mock.calls[0]
    expect(title).toBe(step.title)
    expect(options.description).toBe("It is still running")
    expect(options.description).not.toContain("{")
    expect(options.action.label).toBe("general:_trn_copy")
    const [mutation] = queryClient.getMutationCache().getAll()
    expect(mutation.options.meta?.localErrorToast).toBe(true)
    expect(c.navigate).not.toHaveBeenCalled()
  }
)
