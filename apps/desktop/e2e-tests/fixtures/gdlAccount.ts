import { randomBytes } from "node:crypto"

/**
 * The OID prefix enderium reserves for provisioned test users.
 *
 * Its version nibble is `0`, which RFC 4122 assigns to no UUID version, so no
 * legally versioned identifier from any vendor can collide with it.
 */
export const RESERVED_TEST_OID_PREFIX = "e2e2e2e2-e2e2-0e2e-8e2e-"

export interface ProvisionConfig {
  apiBase: string
  internalToken: string
}

export interface ProvisionedUser {
  token: string
  oid: string
  email: string
  displayName: string
  expiresAt: number
}

/** A fresh OID in the reserved range — 48 bits of per-run entropy. */
export function mintReservedOid(): string {
  return `${RESERVED_TEST_OID_PREFIX}${randomBytes(6).toString("hex")}`
}

/**
 * Provisioning and teardown credentials from the environment, or `null` when
 * they are absent — which selects the mock's standalone mode.
 */
export function readProvisionConfig(): ProvisionConfig | null {
  const apiBase = process.env.TEST_BASE_API
  const internalToken = process.env.E2E_INTERNAL_AUTH_TOKEN

  if (!apiBase || !internalToken) {
    return null
  }

  return { apiBase: apiBase.replace(/\/+$/, ""), internalToken }
}

interface ProvisionResponseBody {
  token: string
  oid: string
  email: string
  display_name: string
  expires_at: number
}

const REQUIRED_PROVISION_FIELDS = [
  "token",
  "oid",
  "email",
  "display_name",
  "expires_at"
] as const

/**
 * Guards a 2xx provisioning response before its fields are read.
 *
 * Without this, a renamed or dropped field on the backend surfaces as
 * `token: undefined` here, `JSON.stringify` silently drops it from whatever
 * gets logged, and the actual failure resurfaces much later as an opaque
 * mid-enrollment deserialize error instead of a named provisioning failure.
 */
function assertProvisionResponseBody(
  body: unknown
): asserts body is ProvisionResponseBody {
  if (typeof body !== "object" || body === null) {
    throw new Error(
      "provisioning succeeded but the response body was not a JSON object"
    )
  }

  const missing = REQUIRED_PROVISION_FIELDS.filter(
    (field) => (body as Record<string, unknown>)[field] === undefined
  )

  if (missing.length > 0) {
    throw new Error(
      `provisioning succeeded but the response is missing: ${missing.join(", ")}`
    )
  }
}

function describeFailure(status: number, body: string): string {
  if (status === 401) {
    return "provisioning rejected the internal token — check E2E_INTERNAL_AUTH_TOKEN"
  }

  if (status === 503) {
    return "provisioning is disabled on this backend — E2E_PROVISIONING_ENABLED is not true"
  }

  return `provisioning failed with ${status}: ${body}`
}

/** Socket and DNS error codes that mean a request never got an answer. */
const NETWORK_ERROR_CODES = new Set([
  "ENOTFOUND",
  "ECONNRESET",
  "ETIMEDOUT",
  "EAI_AGAIN"
])

/**
 * Names the network-level failure behind `error`, or returns `null` for
 * anything else. Node's `fetch` rejects with a `TypeError` "fetch failed"
 * whenever it gets no response at all, with the socket or DNS error as its
 * `cause`; an HTTP error status is a response, so it never lands here.
 */
export function networkFailure(error: unknown): string | null {
  let current: unknown = error
  // Bounded, in case a cause chain ever loops back on itself.
  for (let depth = 0; depth < 5 && current instanceof Error; depth++) {
    const code = (current as { code?: unknown }).code
    if (typeof code === "string" && NETWORK_ERROR_CODES.has(code)) return code
    current = current.cause
  }
  if (error instanceof TypeError && error.message === "fetch failed") {
    return "fetch failed"
  }
  return null
}

/** Pauses before each retry of a request that got no response. A DNS blip
 *  on a CI runner (ENOTFOUND api-test.gdl.gg in teardown, after the test
 *  itself had passed) clears within seconds. */
const NETWORK_RETRY_DELAYS_MS = [1_000, 2_000, 4_000]

/**
 * Runs `send`, and runs it again after each pause in
 * `NETWORK_RETRY_DELAYS_MS` for as long as it fails without getting a
 * response. Any other failure is thrown, and any response, HTTP errors
 * included, is returned, from the attempt that produced it, as before.
 */
async function withNetworkRetry(
  what: string,
  send: () => Promise<Response>
): Promise<Response> {
  for (let attempt = 0; ; attempt++) {
    try {
      return await send()
    } catch (error) {
      const failure = networkFailure(error)
      const delay = NETWORK_RETRY_DELAYS_MS[attempt]
      if (failure === null || delay === undefined) throw error
      console.warn(
        `${what}: no response (${failure}), retrying in ${delay}ms ` +
          `(attempt ${attempt + 2} of ${NETWORK_RETRY_DELAYS_MS.length + 1})`
      )
      await new Promise((resolve) => setTimeout(resolve, delay))
    }
  }
}

async function requestProvision(
  cfg: ProvisionConfig,
  oid: string,
  fetchImpl: typeof fetch
): Promise<Response> {
  // A retry whose earlier attempt did reach the backend meets the row that
  // attempt wrote; `provisionTestUser`'s 409 path covers a backend that
  // refuses that.
  return withNetworkRetry(`provisioning test user ${oid}`, () =>
    fetchImpl(`${cfg.apiBase}/v1/users/internal/provision-test-user`, {
      method: "POST",
      // The internal token is sent bare; these endpoints take no Bearer prefix.
      headers: {
        Authorization: cfg.internalToken,
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ oid })
    })
  )
}

export async function provisionTestUser(
  cfg: ProvisionConfig,
  fetchImpl: typeof fetch = fetch,
  oid: string = mintReservedOid()
): Promise<ProvisionedUser> {
  let response = await requestProvision(cfg, oid, fetchImpl)

  // A 409 means the OID holds a row with an email provisioning did not write.
  // Teardown always succeeds for a reserved OID, so this recovers in one pass.
  if (response.status === 409) {
    await deleteTestUser(cfg, oid, fetchImpl)
    response = await requestProvision(cfg, oid, fetchImpl)
  }

  if (!response.ok) {
    throw new Error(describeFailure(response.status, await response.text()))
  }

  const body: unknown = await response.json()
  assertProvisionResponseBody(body)

  return {
    token: body.token,
    oid: body.oid,
    email: body.email,
    displayName: body.display_name,
    expiresAt: body.expires_at
  }
}

/**
 * Removes a provisioned user. Deleting one that does not exist is a success,
 * so this can run unconditionally in teardown, and so can a retry of a
 * delete whose response was lost.
 */
export async function deleteTestUser(
  cfg: ProvisionConfig,
  oid: string,
  fetchImpl: typeof fetch = fetch
): Promise<void> {
  const response = await withNetworkRetry(`tearing down test user ${oid}`, () =>
    fetchImpl(`${cfg.apiBase}/v1/users/internal/test-user/${oid}`, {
      method: "DELETE",
      headers: { Authorization: cfg.internalToken }
    })
  )

  if (!response.ok) {
    throw new Error(
      `tearing down test user ${oid} failed with ${response.status}`
    )
  }
}
