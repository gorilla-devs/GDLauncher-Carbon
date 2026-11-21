import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { QueryClientProvider } from "@tanstack/solid-query"
import { toast } from "@/test/uiFormsRealButton.fixture"
import { queryClient } from "@/utils/rspcClient"
import ConfirmInstanceDeletion from "./ConfirmInstanceDeletion"
import ConfirmServerDeletion from "./ConfirmServerDeletion"

// The real rspc client and query client, so the global error handler runs
// with the dialog's own mutation options; only the core's reply is faked.
const c = vi.hoisted(() => ({ close: vi.fn(), navigate: vi.fn() }))
vi.mock("@gd/ui", async () => import("@/test/uiFormsRealButton.fixture"))
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
  // The core refuses the delete: rspc's error body, whose message is the
  // core's error with its debug text and backtrace. What the server dialog
  // reads first (the settings and the server list) is answered.
  fetchMock.mockImplementation(async (url: unknown) => ({
    ok: true,
    json: async () =>
      /\/rspc\/(settings\.getSettings|server\.getAllServers)/.test(String(url))
        ? {
            result: {
              type: "response",
              data: String(url).includes("getAllServers")
                ? []
                : { deletionThroughRecycleBin: true }
            }
          }
        : {
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
          }
  }))
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
    Dialog: ConfirmInstanceDeletion,
    confirm: "confirm-instance-deletion",
    procedure: "instance.deleteInstance",
    title: "notifications:_trn_cannot_delete_instance"
  },
  {
    Dialog: ConfirmServerDeletion,
    confirm: "confirm-server-deletion",
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
          <step.Dialog title="Delete" data={{ id: 7, name: "Synthetic" }} />
        </QueryClientProvider>
      ),
      root
    )
    root
      .querySelector<HTMLButtonElement>(`[data-testid="${step.confirm}"]`)!
      .click()
    await vi.waitFor(() => expect(toast.error).toHaveBeenCalled())
    // Let the global handler and any later toast run too.
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(
      fetchMock.mock.calls.filter(([url]) =>
        new RegExp(`/rspc/${step.procedure}$`).test(String(url))
      )
    ).toHaveLength(1)
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
