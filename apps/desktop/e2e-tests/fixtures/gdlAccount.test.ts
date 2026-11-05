import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
  RESERVED_TEST_OID_PREFIX,
  deleteTestUser,
  mintReservedOid,
  networkFailure,
  provisionTestUser
} from "./gdlAccount.js"

const CFG = { apiBase: "https://api-test.invalid", internalToken: "secret" }

const ok = (body: unknown) =>
  new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" }
  })

describe("mintReservedOid", () => {
  it("stays inside the reserved range", () => {
    // enderium rejects anything outside this prefix with a 400 before it reads
    // a row, which is what stops a production OID reaching these endpoints.
    expect(mintReservedOid().startsWith(RESERVED_TEST_OID_PREFIX)).toBe(true)
  })

  it("fills the remaining twelve hex digits", () => {
    expect(mintReservedOid()).toMatch(/^e2e2e2e2-e2e2-0e2e-8e2e-[0-9a-f]{12}$/)
  })

  it("does not repeat", () => {
    const minted = new Set(Array.from({ length: 64 }, () => mintReservedOid()))

    expect(minted.size).toBe(64)
  })
})

describe("provisionTestUser", () => {
  it("sends the bare internal token with no Bearer prefix", async () => {
    const fetchImpl = vi.fn(async () =>
      ok({
        token: "t",
        oid: "o",
        email: "e",
        display_name: "d",
        expires_at: 1
      })
    )

    await provisionTestUser(CFG, fetchImpl as unknown as typeof fetch)

    // @ts-expect-error vitest Mock types don't intersect with fetch signature
    const init = fetchImpl.mock.calls[0][1] as RequestInit
    expect((init.headers as Record<string, string>).Authorization).toBe(
      "secret"
    )
  })

  it("maps the response into camelCase", async () => {
    const fetchImpl = vi.fn(async () =>
      ok({
        token: "t",
        oid: "o",
        email: "e@e.invalid",
        display_name: "e2e_abc",
        expires_at: 99
      })
    )

    const user = await provisionTestUser(
      CFG,
      fetchImpl as unknown as typeof fetch
    )

    expect(user).toEqual({
      token: "t",
      oid: "o",
      email: "e@e.invalid",
      displayName: "e2e_abc",
      expiresAt: 99
    })
  })

  it("tears down and retries once on a 409", async () => {
    // enderium documents this: the OID carries a row whose email provisioning
    // did not write, and teardown-then-provision is the recovery.
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(new Response("conflict", { status: 409 }))
      .mockResolvedValueOnce(ok({ deleted: true }))
      .mockResolvedValueOnce(
        ok({
          token: "t",
          oid: "o",
          email: "e",
          display_name: "d",
          expires_at: 1
        })
      )

    const user = await provisionTestUser(
      CFG,
      fetchImpl as unknown as typeof fetch
    )

    expect(user.token).toBe("t")
    expect(fetchImpl.mock.calls[1][1]).toMatchObject({ method: "DELETE" })
  })

  it("names the feature flag on a 503", async () => {
    const fetchImpl = vi.fn(async () => new Response("", { status: 503 }))

    await expect(
      provisionTestUser(CFG, fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow(/E2E_PROVISIONING_ENABLED/)
  })

  it("names the credential on a 401", async () => {
    const fetchImpl = vi.fn(async () => new Response("", { status: 401 }))

    await expect(
      provisionTestUser(CFG, fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow(/E2E_INTERNAL_AUTH_TOKEN/)
  })

  it("names the missing field when a renamed or dropped field would otherwise surface as undefined", async () => {
    const fetchImpl = vi.fn(async () =>
      ok({
        // `token` is absent, the way a renamed backend field would arrive.
        oid: "o",
        email: "e",
        display_name: "d",
        expires_at: 1
      })
    )

    await expect(
      provisionTestUser(CFG, fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow(/missing: token/)
  })

  it("rejects a response body that isn't a JSON object", async () => {
    const fetchImpl = vi.fn(async () => ok("not an object"))

    await expect(
      provisionTestUser(CFG, fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow(/not a JSON object/)
  })
})

describe("deleteTestUser", () => {
  it("treats a missing user as success", async () => {
    const fetchImpl = vi.fn(async () => ok({ deleted: false }))

    await expect(
      deleteTestUser(CFG, "oid", fetchImpl as unknown as typeof fetch)
    ).resolves.toBeUndefined()
  })

  it("throws on a server error so teardown failures are visible", async () => {
    const fetchImpl = vi.fn(async () => new Response("", { status: 500 }))

    await expect(
      deleteTestUser(CFG, "oid", fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow(/500/)
  })
})

/** What Node's fetch rejects with when it gets no response: a TypeError
 *  "fetch failed" carrying the socket or DNS error as its cause. */
const noResponse = (code: string) =>
  new TypeError("fetch failed", {
    cause: Object.assign(new Error(`getaddrinfo ${code} api-test.invalid`), {
      code
    })
  })

describe("networkFailure", () => {
  it("names the DNS or socket code behind fetch's TypeError", () => {
    for (const code of ["ENOTFOUND", "ECONNRESET", "ETIMEDOUT", "EAI_AGAIN"]) {
      expect(networkFailure(noResponse(code))).toBe(code)
    }
  })

  it("counts any other fetch failed as network-level", () => {
    const refused = noResponse("ECONNREFUSED")

    expect(networkFailure(refused)).toBe("fetch failed")
  })

  it("leaves every other failure alone", () => {
    expect(networkFailure(new Error("boom"))).toBeNull()
    expect(networkFailure(new TypeError("Failed to parse URL"))).toBeNull()
    expect(networkFailure("ENOTFOUND")).toBeNull()
  })
})

describe("retries on a request that got no response", () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.spyOn(console, "warn").mockImplementation(() => {})
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it("deletes once DNS resolves again", async () => {
    // The teardown failure seen on macOS CI: getaddrinfo ENOTFOUND
    // api-test.gdl.gg after the test itself had passed.
    const fetchImpl = vi
      .fn()
      .mockRejectedValueOnce(noResponse("ENOTFOUND"))
      .mockResolvedValueOnce(ok({ deleted: true }))

    const deleted = deleteTestUser(
      CFG,
      "oid",
      fetchImpl as unknown as typeof fetch
    )
    await vi.runAllTimersAsync()

    await expect(deleted).resolves.toBeUndefined()
    expect(fetchImpl).toHaveBeenCalledTimes(2)
  })

  it("provisions once a reset connection recovers", async () => {
    const fetchImpl = vi
      .fn()
      .mockRejectedValueOnce(noResponse("ECONNRESET"))
      .mockResolvedValueOnce(
        ok({
          token: "t",
          oid: "o",
          email: "e",
          display_name: "d",
          expires_at: 1
        })
      )

    const provisioned = provisionTestUser(
      CFG,
      fetchImpl as unknown as typeof fetch
    )
    await vi.runAllTimersAsync()

    await expect(provisioned).resolves.toMatchObject({ token: "t" })
    expect(fetchImpl).toHaveBeenCalledTimes(2)
  })

  it("gives up after three retries with the network error", async () => {
    const fetchImpl = vi.fn(async () => {
      throw noResponse("EAI_AGAIN")
    })

    const failed = expect(
      deleteTestUser(CFG, "oid", fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow("fetch failed")
    await vi.runAllTimersAsync()

    await failed
    expect(fetchImpl).toHaveBeenCalledTimes(4)
  })

  it("does not retry an HTTP error", async () => {
    const fetchImpl = vi.fn(async () => new Response("", { status: 502 }))

    await expect(
      deleteTestUser(CFG, "oid", fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow(/502/)
    expect(fetchImpl).toHaveBeenCalledTimes(1)
  })

  it("does not retry a failure that is not network-level", async () => {
    const fetchImpl = vi.fn(async () => {
      throw new Error("boom")
    })

    await expect(
      provisionTestUser(CFG, fetchImpl as unknown as typeof fetch)
    ).rejects.toThrow("boom")
    expect(fetchImpl).toHaveBeenCalledTimes(1)
  })
})
