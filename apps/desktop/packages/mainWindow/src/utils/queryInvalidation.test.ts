import { it, expect, vi } from "vitest"
import { QueryClient } from "@tanstack/query-core"
import { parseInvalidateFrame, invalidationKey } from "./queryInvalidation"

it("no-input frames invalidate the installed query without external fixtures", async () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } }
  })
  try {
    client.setQueryData(
      ["account.getAccounts"],
      [{ uuid: "a", status: "expired" }]
    )
    const frame = parseInvalidateFrame(
      '{"key":"account.getAccounts","args":null}'
    )!
    await client.invalidateQueries({
      queryKey: invalidationKey(frame),
      refetchType: "none"
    })
    expect(client.getQueryState(["account.getAccounts"])?.isInvalidated).toBe(
      true
    )
    expect(
      await client.fetchQuery({
        queryKey: ["account.getAccounts"],
        queryFn: async () => "fresh"
      })
    ).toBe("fresh")
  } finally {
    client.clear()
  }
})

it("UUID routes remain narrow", async () => {
  const client = new QueryClient()
  try {
    client.setQueryData(["account.getAccounts"], [])
    await client.invalidateQueries({
      queryKey: invalidationKey({ key: "account.getAccounts", args: "a" }),
      refetchType: "none"
    })
    expect(client.getQueryState(["account.getAccounts"])?.isInvalidated).toBe(
      false
    )
  } finally {
    client.clear()
  }
})

it("malformed frames are dropped with the expected diagnostic", () => {
  const warning = vi.spyOn(console, "warn").mockImplementation(() => {})
  try {
    expect(parseInvalidateFrame("invalid")).toBeUndefined()
    expect(warning).toHaveBeenCalledTimes(1)
    expect(warning).toHaveBeenCalledWith(
      "Invalidations channel: dropping unparsable frame",
      expect.any(SyntaxError)
    )
  } finally {
    warning.mockRestore()
  }
})
