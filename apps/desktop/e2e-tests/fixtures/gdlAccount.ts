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

function describeFailure(status: number, body: string): string {
  if (status === 401) {
    return "provisioning rejected the internal token — check E2E_INTERNAL_AUTH_TOKEN"
  }

  if (status === 503) {
    return "provisioning is disabled on this backend — E2E_PROVISIONING_ENABLED is not true"
  }

  return `provisioning failed with ${status}: ${body}`
}

async function requestProvision(
  cfg: ProvisionConfig,
  oid: string,
  fetchImpl: typeof fetch
): Promise<Response> {
  return fetchImpl(`${cfg.apiBase}/v1/users/internal/provision-test-user`, {
    method: "POST",
    // The internal token is sent bare; these endpoints take no Bearer prefix.
    headers: {
      Authorization: cfg.internalToken,
      "Content-Type": "application/json"
    },
    body: JSON.stringify({ oid })
  })
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

  const body = await response.json()

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
 * so this can run unconditionally in teardown.
 */
export async function deleteTestUser(
  cfg: ProvisionConfig,
  oid: string,
  fetchImpl: typeof fetch = fetch
): Promise<void> {
  const response = await fetchImpl(
    `${cfg.apiBase}/v1/users/internal/test-user/${oid}`,
    { method: "DELETE", headers: { Authorization: cfg.internalToken } }
  )

  if (!response.ok) {
    throw new Error(
      `tearing down test user ${oid} failed with ${response.status}`
    )
  }
}
