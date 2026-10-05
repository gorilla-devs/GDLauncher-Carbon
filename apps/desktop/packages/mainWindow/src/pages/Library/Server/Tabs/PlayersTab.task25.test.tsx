import { afterEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"

const control = vi.hoisted(() => ({
  lists: null as any,
  mutation: vi.fn(),
  refetch: vi.fn()
}))
vi.mock("@gd/ui", () => import("@/test/task25/ui"))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({ useParams: () => ({ id: "7" }) }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: (opts: any) => ({
      get data() {
        return control.lists[opts().queryKey[0]]
      },
      refetch: control.refetch
    }),
    createMutation: (opts: any) => ({
      mutateAsync: (payload: any) =>
        control.mutation(opts().mutationKey[0], payload),
      isPending: false
    })
  }
}))
import PlayersTab from "./PlayersTab"
let dispose: () => void
let root: HTMLDivElement
afterEach(() => {
  dispose?.()
  root?.remove()
  vi.clearAllMocks()
})
function mount(ips: any[]) {
  const [lists] = createStore({
    "server.getWhitelist": [{ name: "SyntheticPlayer", uuid: "player-id" }],
    "server.getOps": [],
    "server.getBannedPlayers": [],
    "server.getBannedIps": ips
  })
  control.lists = lists
  control.mutation.mockResolvedValue(undefined)
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <PlayersTab />, root)
}
function bannedTab() {
  const tab = [...root.querySelectorAll("button")].find((button) =>
    button.textContent?.includes("_trn_server_players_banned_ips")
  )!
  tab.click()
}
test("visible banned-IP rows emit no remote resource while retaining local address, reason and exact unban payload", async () => {
  mount([
    { ip: "192.0.2.42", reason: "Synthetic reason" },
    { ip: "2001:db8::42", reason: "IPv6 reason" }
  ])
  expect(root.querySelector("img")?.getAttribute("src")).toBe(
    "https://mc-heads.net/avatar/SyntheticPlayer/24"
  )
  bannedTab()
  expect(root.textContent).toContain("192.0.2.42")
  expect(root.textContent).toContain("2001:db8::42")
  expect(root.textContent).toContain("Synthetic reason")
  expect(root.querySelectorAll("img, [src], [href]")).toHaveLength(0)
  const remove = [...root.querySelectorAll("button")].find((button) =>
    button.textContent?.includes("_trn_server_players_unban")
  )!
  remove.click()
  await Promise.resolve()
  expect(control.mutation).toHaveBeenCalledWith("server.unbanIp", {
    serverId: 7,
    ip: "192.0.2.42"
  })
  expect(control.refetch).toHaveBeenCalledOnce()
})
test("empty banned-IP list mounts no resource or unban row", () => {
  mount([])
  bannedTab()
  expect(root.querySelectorAll("img, [src], [href]")).toHaveLength(0)
  expect(root.textContent).toContain("_trn_server_players_no_entries")
})
