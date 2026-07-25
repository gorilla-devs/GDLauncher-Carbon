import { createPublicKey, createVerify } from "node:crypto"
import {
  createServer,
  request,
  type IncomingHttpHeaders,
  type Server
} from "node:http"
import { AddressInfo } from "node:net"
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

describe("gdl token exchange", () => {
  it("answers the exchange with the provisioned token", async () => {
    // enderium validates a real Microsoft id_token against Microsoft's live
    // JWKS, so this hop can never reach api-test with a mock-minted token.
    const res = await fetch(`${mock.url}/gdl/v1/auth/token`, {
      method: "POST",
      headers: { authorization: "Bearer mock-id-token" }
    })
    const body = await res.json()

    expect(res.status).toBe(200)
    expect(body.access_token).toBe("gdl-token-under-test")
    expect(body.token_type).toBe("Bearer")
    expect(body.expires_at).toBeGreaterThan(Math.floor(Date.now() / 1000))
  })

  it("serves the standalone stubs when no api-test base is configured", async () => {
    const res = await fetch(`${mock.url}/gdl/v1/users/user`)

    expect(res.status).toBe(200)
    expect(await res.json()).toHaveProperty("microsoft_oid")
  })

  it("names the route when a gdl call has no stub and no proxy", async () => {
    const res = await fetch(`${mock.url}/gdl/v1/instance-share/quota`)

    expect(res.status).toBe(501)
    expect(await res.text()).toContain("/v1/instance-share/quota")
  })
})

interface UpstreamRequest {
  method?: string
  url?: string
  body: string
  headers: IncomingHttpHeaders
}

describe("gdl proxy to api-test", () => {
  let upstream: Server
  let upstreamPort: number
  // Written by the upstream's own request handler below, from inside the
  // "strips hop-by-hop headers" test's raw http.request callbacks — a
  // closure TypeScript's flow analysis doesn't see into. Resetting this
  // with a same-type cast (rather than a bare `= undefined`) keeps it typed
  // as `UpstreamRequest | undefined` at the read site instead of narrowing
  // it to `undefined` (and then `never` on property access) for the rest of
  // the test, which would silently defeat the assertions that read it.
  let upstreamRequest: UpstreamRequest | undefined
  let proxied: MockServer

  beforeEach(async () => {
    upstream = createServer((req, res) => {
      const chunks: Buffer[] = []
      req.on("data", (chunk) => chunks.push(chunk))
      req.on("end", () => {
        upstreamRequest = {
          method: req.method,
          url: req.url,
          body: Buffer.concat(chunks).toString("utf8"),
          headers: req.headers
        }
        res.writeHead(201, { "content-type": "application/json" })
        res.end(JSON.stringify({ echoed: true }))
      })
    })
    await new Promise<void>((resolve) =>
      upstream.listen(0, "127.0.0.1", resolve)
    )

    upstreamPort = (upstream.address() as AddressInfo).port
    proxied = await startMockServer({
      identity: identityFromOid(OID, `e2e-${OID}@e2e.invalid`, "e2e_6789ab"),
      gdlToken: "gdl-token-under-test",
      apiTestBase: `http://127.0.0.1:${upstreamPort}`
    })
  })

  afterEach(async () => {
    await proxied.close()
    await new Promise<void>((resolve) => upstream.close(() => resolve()))
  })

  it("forwards method, path, and body to the upstream and relays its response unchanged", async () => {
    const res = await fetch(`${proxied.url}/gdl/v1/instance-share/quota`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ hello: "world" })
    })

    expect(res.status).toBe(201)
    expect(await res.json()).toEqual({ echoed: true })

    expect(upstreamRequest?.method).toBe("POST")
    // The mock strips the "/gdl" prefix before forwarding, since api-test
    // itself doesn't know about it.
    expect(upstreamRequest?.url).toBe("/v1/instance-share/quota")
    expect(upstreamRequest?.body).toBe(JSON.stringify({ hello: "world" }))
  })

  it("forwards a bodyless GET without hanging on a body that will never arrive", async () => {
    const res = await fetch(`${proxied.url}/gdl/v1/instance-share/quota`)

    expect(res.status).toBe(201)
    expect(upstreamRequest?.method).toBe("GET")
    expect(upstreamRequest?.body).toBe("")
  })

  it("strips hop-by-hop headers instead of relaying them to the upstream", async () => {
    // A plain `fetch()` can't set most of these — building the request by
    // hand is what actually exercises the strip logic in forwardToApiTest,
    // since undici's own fetch either silently drops or outright refuses to
    // send several of them itself.
    const body = JSON.stringify({ hello: "world" })
    const { port: proxiedPort } = proxied

    // If forwardToApiTest fails to strip a header `fetch` itself rejects on
    // (transfer-encoding, upgrade), the promise below still resolves — it's
    // the mock's own `handle().catch()` answering 500, not a hang — so a
    // status assertion is what actually catches that case; leftover state in
    // `upstreamRequest` from an earlier call would make a bare header check
    // pass by accident.
    function rawPost(
      headers: Record<string, string | number>
    ): Promise<number> {
      return new Promise((resolve, reject) => {
        const req = request(
          {
            hostname: "127.0.0.1",
            port: proxiedPort,
            method: "POST",
            path: "/gdl/v1/instance-share/quota",
            headers
          },
          (res) => {
            res.on("data", () => {})
            res.on("end", () => resolve(res.statusCode ?? 0))
          }
        )
        req.on("error", reject)
        req.end(body)
      })
    }

    upstreamRequest = undefined as UpstreamRequest | undefined
    const statusA = await rawPost({
      "content-type": "application/json",
      "content-length": Buffer.byteLength(body),
      // A value `fetch` would never independently produce (its own default
      // for an outgoing HTTP/1.1 call is "keep-alive"), so seeing it survive
      // on the upstream side would mean this request's original header
      // leaked through unstripped.
      connection: "close",
      "keep-alive": "timeout=5",
      te: "trailers",
      "proxy-authorization": "Basic mock-proxy-creds",
      "proxy-authenticate": "Basic realm=mock"
    })

    expect(statusA).toBe(201)
    // `host` must name the upstream the mock dialed, not the mock itself.
    expect(upstreamRequest?.headers.host).toBe(`127.0.0.1:${upstreamPort}`)
    // `fetch` always sets its own "connection" and "content-length" for an
    // outgoing request with a known-length body, so their absence isn't the
    // right signal — what matters is that neither carries this request's
    // original values through: a leaked "close" would mean the upstream
    // connection gets torn down after one call, and a leaked, unrecomputed
    // content-length is exactly what causes the truncated/hung requests this
    // function exists to avoid.
    expect(upstreamRequest?.headers.connection).not.toBe("close")
    expect(upstreamRequest?.headers["content-length"]).toBe(
      String(Buffer.byteLength(body))
    )
    for (const stripped of [
      "keep-alive",
      "te",
      "proxy-authorization",
      "proxy-authenticate"
    ]) {
      expect(upstreamRequest?.headers[stripped]).toBeUndefined()
    }

    // transfer-encoding, upgrade, and trailer get their own request: any of
    // them alongside a fixed content-length produces an ambiguous or
    // trailer-without-chunking request that Node itself refuses to send, so
    // this has to be a second, chunked call.
    upstreamRequest = undefined as UpstreamRequest | undefined
    const statusB = await rawPost({
      "content-type": "application/json",
      "transfer-encoding": "chunked",
      upgrade: "h2c",
      trailer: "X-Foo"
    })

    // A leaked transfer-encoding or upgrade header makes the forwarded
    // `fetch` call reject synchronously; the mock's catch-all then answers
    // 500 instead of relaying the upstream's 201, which is exactly the
    // regression this assertion exists to catch.
    expect(statusB).toBe(201)
    expect(upstreamRequest?.headers["transfer-encoding"]).toBeUndefined()
    expect(upstreamRequest?.headers.upgrade).toBeUndefined()
    expect(upstreamRequest?.headers.trailer).toBeUndefined()
  })
})
