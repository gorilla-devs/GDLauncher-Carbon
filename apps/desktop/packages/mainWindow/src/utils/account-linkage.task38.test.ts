import { expect, it } from "vitest"
import { FlowControllerImpl } from "../pages/Login/flow/FlowController"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import ts from "typescript"

// Use actual production method bodies with injected in-memory dependencies;
// do not construct the controller, whose constructor starts initialization.
function fixture(gdlAccountId: string | null = null) {
  const trace: any[] = []
  let state: any
  const owner: any = Object.create(FlowControllerImpl.prototype)
  owner.data = {
    activeUuid: "synthetic-uuid",
    gdlAccountId,
    accounts: [],
    isFirstLaunch: false
  }
  owner.setState = (s: any) => {
    state = s
    trace.push(["state", s.phase])
  }
  owner.state = () => state
  owner.accountsQuery = {
    refetch: async () => ({ data: [{ uuid: "synthetic-uuid" }] })
  }
  owner.saveGdlAccountMutation = {
    mutateAsync: async (uuid: string) => trace.push(["save", uuid])
  }
  owner.rspcContext = {
    client: {
      query: async (key: any) => {
        trace.push(["query", key])
        return { isEmailVerified: true, displayName: "Synthetic" }
      }
    },
    queryClient: {
      refetchQueries: async (key: any) => trace.push(["refetch", key])
    }
  }
  return { owner, trace, state: () => state }
}
it("GPT0266 production found-account/link methods retain active UUID, settings refresh and linked identity", async () => {
  const f = fixture()
  await f.owner.checkExistingAccount()
  expect(f.state().step.gdlAccount.type).toBe("found-existing")
  await f.owner.linkExistingGDLAccount(f.state().step.gdlAccount.data)
  expect(f.trace.slice(-2)).toEqual([
    ["save", "synthetic-uuid"],
    ["refetch", { queryKey: ["settings.getSettings"] }]
  ])
  expect(f.owner.data.gdlAccountId).toBe("synthetic-uuid")
})
it("GPT0266 already-linked account exits without a found/restore prompt", async () => {
  const f = fixture("synthetic-uuid")
  await f.owner.checkExistingAccount()
  expect(f.state().phase).toBe("exiting")
  expect(f.trace.some((row) => row[0] === "save")).toBe(false)
})
// Source-derived exact handler expression; no App/AuthFlow render or transport.
const path = resolve(
  process.cwd(),
  "packages/mainWindow/src/pages/Login/AuthFlow.tsx"
)
const source = readFileSync(path, "utf8")
const ast = ts.createSourceFile(
  path,
  source,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TSX
)
let expression = ""
function visit(node: ts.Node) {
  if (
    ts.isVariableDeclaration(node) &&
    node.name.getText(ast) === "handleSyncExistingAccount"
  )
    expression = node.initializer!.getText(ast)
  ts.forEachChild(node, visit)
}
visit(ast)
const script = ts.transpileModule(`const handler = ${expression};`, {
  compilerOptions: { target: ts.ScriptTarget.ESNext }
}).outputText
// eslint-disable-next-line @typescript-eslint/no-implied-eval -- evaluates the extracted handler under test
const handler = new Function(
  "getCurrentStep",
  "setButtonLoading",
  "flow",
  `${script}; return handler;`
)
it.each([
  { type: "welcome" },
  { type: "gdl-account", gdlAccount: { type: "none" } }
])(
  "GPT0266 exact Sync handler rejects wrong step/state %j without effects",
  async (step) => {
    const f = fixture(),
      loading: boolean[] = []
    await handler(
      () => step,
      (value: boolean) => loading.push(value),
      f.owner
    )()
    expect(f.trace).toEqual([])
    expect(loading).toEqual([])
  }
)
it("GPT0266 exact Sync handler links found account then exits; no restoration request exists in trace", async () => {
  const f = fixture(),
    loading: boolean[] = []
  await handler(
    () => ({
      type: "gdl-account",
      gdlAccount: { type: "found-existing", data: {} }
    }),
    (value: boolean) => loading.push(value),
    f.owner
  )()
  expect(loading).toEqual([true])
  expect(f.trace).toEqual([
    ["save", "synthetic-uuid"],
    ["refetch", { queryKey: ["settings.getSettings"] }],
    ["state", "exiting"]
  ])
})
