import { createPublicKey, createVerify } from "node:crypto"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import { identityFromOid, startMockServer, type MockServer } from "./server.js"

const OID = "e2e2e2e2-e2e2-0e2e-8e2e-0123456789ab"

let mock: MockServer

beforeEach(async () => {
  mock = await startMockServer({
    identity: identityFromOid(OID, `e2e-${OID}@e2e.invalid`, "e2e_6789ab"),
    gdlToken: "gdl-token-under-test"
  })
})

afterEach(async () => {
  await mock.close()
})

describe("device code flow", () => {
  it("issues a device code", async () => {
    const res = await fetch(
      `${mock.url}/ms/consumers/oauth2/v2.0/devicecode?client_id=x&scope=y`
    )
    const body = await res.json()

    expect(res.status).toBe(200)
    expect(body.device_code).toBeTruthy()
    expect(body.user_code).toBeTruthy()
    expect(body.verification_uri).toBeTruthy()
    expect(typeof body.expires_in).toBe("number")
    expect(typeof body.interval).toBe("number")
  })

  it("answers authorization_pending until approved", async () => {
    const res = await fetch(`${mock.url}/ms/consumers/oauth2/v2.0/token`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=d"
    })

    // Carbon reads a 400 body's `error` field to decide whether to keep
    // polling; any other status aborts enrollment.
    expect(res.status).toBe(400)
    expect((await res.json()).error).toBe("authorization_pending")
  })

  it("issues tokens once approved", async () => {
    mock.approve()

    const res = await fetch(`${mock.url}/ms/consumers/oauth2/v2.0/token`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=d"
    })
    const body = await res.json()

    expect(res.status).toBe(200)
    expect(body.access_token).toBeTruthy()
    expect(body.refresh_token).toBeTruthy()
    expect(body.id_token.split(".")).toHaveLength(3)
    expect(body.expires_in).toBeGreaterThan(0)
  })

  it("carries the identity in the id_token claims", async () => {
    mock.approve()

    const res = await fetch(`${mock.url}/ms/consumers/oauth2/v2.0/token`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=d"
    })
    const claims = JSON.parse(
      Buffer.from(
        (await res.json()).id_token.split(".")[1],
        "base64url"
      ).toString("utf8")
    )

    expect(claims.oid).toBe(OID)
    expect(claims.email).toBe(`e2e-${OID}@e2e.invalid`)
    // ensure_gdl_auth_token refreshes the Microsoft account when the id_token
    // is expired, which would send it to a token endpoint mid-test.
    expect(claims.exp * 1000).toBeGreaterThan(Date.now())
  })

  it("serves the refresh grant", async () => {
    const res = await fetch(`${mock.url}/ms/consumers/oauth2/v2.0/token`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "grant_type=refresh_token&refresh_token=r"
    })
    const body = await res.json()

    // MsAuth::refresh's RefreshResponse needs all three to build a full
    // account row, not just the access token.
    expect(res.status).toBe(200)
    expect(body.access_token).toBeTruthy()
    expect(body.id_token).toBeTruthy()
    expect(body.refresh_token).toBeTruthy()
  })
})

describe("xbox chain", () => {
  it("returns an xbl token", async () => {
    const res = await fetch(`${mock.url}/xbl/user/authenticate`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ RelyingParty: "http://auth.xboxlive.com" })
    })

    expect(res.status).toBe(200)
    expect((await res.json()).Token).toBeTruthy()
  })

  it("returns an xsts token with a user hash", async () => {
    const res = await fetch(`${mock.url}/xsts/xsts/authorize`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ RelyingParty: "rp://api.minecraftservices.com/" })
    })
    const body = await res.json()

    expect(res.status).toBe(200)
    expect(body.Token).toBeTruthy()
    expect(body.DisplayClaims.xui[0].uhs).toBeTruthy()
  })
})

describe("minecraft services", () => {
  it("returns a minecraft access token", async () => {
    const res = await fetch(`${mock.url}/mc/authentication/login_with_xbox`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({})
    })
    const body = await res.json()

    expect(res.status).toBe(200)
    expect(body.access_token).toBeTruthy()
    expect(body.expires_in).toBeGreaterThan(0)
  })

  it("returns an entitlement signed with the run's key", async () => {
    const res = await fetch(`${mock.url}/mc/entitlements/mcstore`)
    const { signature } = await res.json()

    const [header, payload, sig] = signature.split(".")
    const verifier = createVerify("RSA-SHA256")
    verifier.update(`${header}.${payload}`)

    expect(
      verifier.verify(
        createPublicKey(mock.publicKeyPem),
        Buffer.from(sig, "base64url")
      )
    ).toBe(true)

    const claims = JSON.parse(
      Buffer.from(payload, "base64url").toString("utf8")
    )
    // The launcher looks for this exact product name to decide ownership.
    expect(claims.entitlements).toContainEqual({ name: "product_minecraft" })
    // jsonwebtoken's default Validation rejects a token with no `exp`
    // before the launcher ever inspects the entitlements list.
    expect(claims.exp).toBeGreaterThan(Math.floor(Date.now() / 1000))
  })

  it("returns a profile with an ACTIVE skin", async () => {
    const res = await fetch(`${mock.url}/mc/minecraft/profile`)
    const body = await res.json()

    expect(res.status).toBe(200)
    expect(body.id).toBeTruthy()
    expect(body.name).toBe("e2e_6789ab")
    // get_profile picks the skin whose state is exactly "ACTIVE".
    expect(
      body.skins.some((s: { state: string }) => s.state === "ACTIVE")
    ).toBe(true)
  })
})

describe("control plane", () => {
  it("records the routes that were called", async () => {
    await fetch(`${mock.url}/mc/minecraft/profile`)

    expect(mock.requests()).toContain("GET /mc/minecraft/profile")
  })

  it("makes a named route fail once", async () => {
    mock.failNext("/mc/minecraft/profile", 401, '{"error":"nope"}')

    const failed = await fetch(`${mock.url}/mc/minecraft/profile`)
    expect(failed.status).toBe(401)

    const recovered = await fetch(`${mock.url}/mc/minecraft/profile`)
    expect(recovered.status).toBe(200)
  })

  it("answers an unknown route with 501 naming it", async () => {
    const res = await fetch(`${mock.url}/mc/nope`)

    expect(res.status).toBe(501)
    expect(await res.text()).toContain("/mc/nope")
  })

  it("survives a malformed control-plane body instead of taking the process down", async () => {
    const broken = await fetch(`${mock.url}/__control/fail`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "not json"
    })

    expect(broken.status).toBeGreaterThanOrEqual(500)

    // The assertion that matters: the server process is still alive and
    // answering, not just that the bad request itself got a 5xx.
    const recovered = await fetch(`${mock.url}/mc/minecraft/profile`)
    expect(recovered.status).toBe(200)
  })
})

describe("identityFromOid", () => {
  it("derives a stable minecraft uuid from the oid", () => {
    const a = identityFromOid(OID, "e@e.invalid", "n")
    const b = identityFromOid(OID, "e@e.invalid", "n")

    expect(a.mcUuid).toBe(b.mcUuid)
    expect(a.mcUuid).toMatch(/^[0-9a-f]{32}$/)
  })

  it("gives different oids different minecraft uuids", () => {
    const a = identityFromOid(OID, "e@e.invalid", "n")
    const b = identityFromOid(
      "e2e2e2e2-e2e2-0e2e-8e2e-ba9876543210",
      "e@e.invalid",
      "n"
    )

    expect(a.mcUuid).not.toBe(b.mcUuid)
  })
})
