import { createSign, generateKeyPairSync } from "node:crypto"

export const SECONDS_IN_A_DAY = 86_400

export function base64url(input: string | Buffer): string {
  return Buffer.from(input).toString("base64url")
}

/**
 * A JWT with a well-formed header and payload and a placeholder signature.
 *
 * Carbon reads the Microsoft tokens with `jwt::Token::parse_unverified`, which
 * needs three segments and ignores the signature entirely. The entitlement
 * response is the one place a genuine signature is required — see
 * `signEntitlementJwt`.
 */
export function unsignedJwt(claims: Record<string, unknown>): string {
  const header = base64url(JSON.stringify({ alg: "RS256", typ: "JWT" }))
  const payload = base64url(JSON.stringify(claims))

  return `${header}.${payload}.${base64url("mock-signature")}`
}

/**
 * A fresh RSA keypair for this run.
 *
 * Generated rather than committed: the private half signs entitlements that a
 * build carrying the `e2e` feature will trust, so it must not exist anywhere a
 * shipped binary could meet it.
 */
export function generateEntitlementKeypair(): {
  publicKeyPem: string
  privateKeyPem: string
} {
  const { publicKey, privateKey } = generateKeyPairSync("rsa", {
    modulusLength: 2048,
    // SPKI is the shape jsonwebtoken's DecodingKey::from_rsa_pem parses.
    publicKeyEncoding: { type: "spki", format: "pem" },
    privateKeyEncoding: { type: "pkcs8", format: "pem" }
  })

  return { publicKeyPem: publicKey, privateKeyPem: privateKey }
}

/**
 * An RS256-signed JWT for the entitlement response.
 *
 * `McEntitlement::mojang_jwt_key` verifies this offline, so redirecting hosts
 * is not enough on its own — the launcher must also be started with the
 * matching public key.
 */
export function signEntitlementJwt(
  privateKeyPem: string,
  claims: Record<string, unknown>
): string {
  const header = base64url(JSON.stringify({ alg: "RS256", typ: "JWT" }))
  const payload = base64url(JSON.stringify(claims))

  const signer = createSign("RSA-SHA256")
  signer.update(`${header}.${payload}`)

  return `${header}.${payload}.${signer.sign(privateKeyPem, "base64url")}`
}
