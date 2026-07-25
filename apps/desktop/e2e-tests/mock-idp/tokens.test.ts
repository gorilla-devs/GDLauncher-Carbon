import { createPublicKey, createVerify } from "node:crypto"
import { describe, expect, it } from "vitest"
import {
  base64url,
  generateEntitlementKeypair,
  signEntitlementJwt,
  unsignedJwt
} from "./tokens.js"

describe("base64url", () => {
  it("uses the url alphabet and strips padding", () => {
    // '?' and '~' encode to bytes that produce '+' and '/' in standard base64.
    expect(base64url("??~~")).toBe("Pz9-fg")
  })
})

describe("unsignedJwt", () => {
  it("produces three dot-separated segments", () => {
    // Carbon parses these with jwt::Token::parse_unverified, which requires
    // exactly three segments and never checks the signature.
    const parts = unsignedJwt({ oid: "abc" }).split(".")

    expect(parts).toHaveLength(3)
    expect(parts[2]).not.toBe("")
  })

  it("round-trips the claims through the payload segment", () => {
    const token = unsignedJwt({ oid: "abc", exp: 42 })
    const payload = JSON.parse(
      Buffer.from(token.split(".")[1], "base64url").toString("utf8")
    )

    expect(payload).toEqual({ oid: "abc", exp: 42 })
  })

  it("declares RS256 in the header so the shape matches a real token", () => {
    const header = JSON.parse(
      Buffer.from(unsignedJwt({}).split(".")[0], "base64url").toString("utf8")
    )

    expect(header.alg).toBe("RS256")
    expect(header.typ).toBe("JWT")
  })
})

describe("signEntitlementJwt", () => {
  it("produces a signature the matching public key verifies", () => {
    const { publicKeyPem, privateKeyPem } = generateEntitlementKeypair()
    const token = signEntitlementJwt(privateKeyPem, {
      entitlements: [{ name: "product_minecraft" }]
    })

    const [header, payload, signature] = token.split(".")
    const verifier = createVerify("RSA-SHA256")
    verifier.update(`${header}.${payload}`)

    expect(
      verifier.verify(
        createPublicKey(publicKeyPem),
        Buffer.from(signature, "base64url")
      )
    ).toBe(true)
  })

  it("is rejected by an unrelated public key", () => {
    const a = generateEntitlementKeypair()
    const b = generateEntitlementKeypair()
    const token = signEntitlementJwt(a.privateKeyPem, { entitlements: [] })

    const [header, payload, signature] = token.split(".")
    const verifier = createVerify("RSA-SHA256")
    verifier.update(`${header}.${payload}`)

    expect(
      verifier.verify(
        createPublicKey(b.publicKeyPem),
        Buffer.from(signature, "base64url")
      )
    ).toBe(false)
  })

  it("emits a SPKI public key, which is what from_rsa_pem expects", () => {
    expect(generateEntitlementKeypair().publicKeyPem).toContain(
      "-----BEGIN PUBLIC KEY-----"
    )
  })
})
