import { it, expect } from "vitest"
import { QueryClient } from "@tanstack/query-core"
import { readFileSync } from "node:fs"
import { parseInvalidateFrame, invalidationKey } from "./queryInvalidation"

// Explicit cross-language check; selected only by vitest.account-integration.config.mjs
// and run by `pnpm test:account-contract`, which produces the fixture first.
// Absence, invalid JSON, wrong frames or a wrong count must fail, never skip.
it("captured production account events invalidate the installed no-input query", async () => {
  const path = process.env.TASK19_ACCOUNT_EVENTS
  if (!path)
    throw new Error(
      "required captured backend event fixture is absent: run `pnpm test:account-contract` from the repository root"
    )
  const events = JSON.parse(readFileSync(path, "utf8"))
  expect(Array.isArray(events)).toBe(true)
  expect(events).toHaveLength(5)
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } }
  })
  try {
    for (const event of events) {
      expect(event).toEqual({ key: "account.getAccounts", args: null })
      client.setQueryData(
        ["account.getAccounts"],
        [{ uuid: "a", status: "expired" }]
      )
      const frame = parseInvalidateFrame(JSON.stringify(event))!
      await client.invalidateQueries({
        queryKey: invalidationKey(frame),
        refetchType: "none"
      })
      expect(client.getQueryState(["account.getAccounts"])?.isInvalidated).toBe(
        true
      )
      const updated = await client.fetchQuery({
        queryKey: ["account.getAccounts"],
        queryFn: async () => [{ uuid: "a", status: "ok" }]
      })
      expect(updated[0].status).toBe("ok")
    }
  } finally {
    client.clear()
  }
})
