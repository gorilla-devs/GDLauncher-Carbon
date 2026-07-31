import { createHash } from "node:crypto"
import {
  createServer,
  type IncomingMessage,
  type Server,
  type ServerResponse
} from "node:http"
import { AddressInfo } from "node:net"
import { forwardToApiTest, standaloneStub } from "./proxy.js"
import {
  SECONDS_IN_A_DAY,
  generateEntitlementKeypair,
  signEntitlementJwt,
  unsignedJwt
} from "./tokens.js"

export interface MockIdentity {
  oid: string
  email: string
  displayName: string
  mcUuid: string
  mcUsername: string
}

export interface MockServerOptions {
  identity: MockIdentity
  /** The GDL JWT served in place of a real token exchange. */
  gdlToken: string
  /** When set, unhandled `/gdl/v1/*` calls are proxied here. */
  apiTestBase?: string
}

export interface MockServer {
  url: string
  port: number
  publicKeyPem: string
  approve(): void
  failNext(route: string, status: number, body: string): void
  requests(): string[]
  close(): Promise<void>
}

/**
 * A Minecraft identity derived deterministically from a provisioned OID.
 *
 * The Minecraft UUID is a hash of the OID rather than a random value so a
 * rerun against the same provisioned user produces the same account row.
 */
export function identityFromOid(
  oid: string,
  email: string,
  displayName: string
): MockIdentity {
  return {
    oid,
    email,
    displayName,
    mcUuid: createHash("sha256").update(oid).digest("hex").slice(0, 32),
    mcUsername: displayName
  }
}

function readBody(req: IncomingMessage): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    req.on("data", (chunk) => chunks.push(chunk))
    req.on("end", () => resolve(Buffer.concat(chunks)))
    req.on("error", reject)
  })
}

function json(res: ServerResponse, status: number, body: unknown): void {
  const payload = JSON.stringify(body)
  res.writeHead(status, {
    "content-type": "application/json",
    "content-length": Buffer.byteLength(payload)
  })
  res.end(payload)
}

/**
 * electron-updater's generic provider asks for `<channel>-<platform>.yml`,
 * with the arch appended on anything but x64 — `latest-linux.yml`,
 * `latest-linux-arm64.yml`, and the `beta-`/`alpha-` variants the release
 * channel picks between (`autoUpdater.ts` sets `autoUpdater.channel`). All of
 * them get the same answer here; which one a run asks for depends on the host
 * and the settings row, and none of that changes what this feed is for.
 */
const UPDATE_CHANNEL_FILE = /^\/updates\/[a-z]+-[a-z0-9-]+\.yml$/

/**
 * A channel file reporting the lowest version semver has.
 *
 * The app under test compares this against its own version and finds nothing
 * newer, so `checkForUpdates` resolves to `update-not-available` instead of
 * erroring — which is the point: a failed check raises an 8-second error
 * toast (`utils/updater.tsx`'s `case "error"`) that sits over the login
 * screen's continue button, and Playwright's click waits out the whole toast
 * before the button can receive the pointer event.
 *
 * `0.0.0` rather than the packaged version because nothing here knows what
 * that is: electron-builder stamps it into the app at package time. Reporting
 * the floor works against any build. It reads as a downgrade rather than an
 * update, which is equally "nothing to do" while `allowDowngrade` is false —
 * and it is false on the stable channel, since `autoUpdater.ts` derives it
 * from `selectedChannelNumber < currentChannelNumber`.
 *
 * The `files`/`path`/`sha512`/`releaseDate` fields are here because
 * electron-updater parses this into an `UpdateInfo` before it compares
 * versions. A file missing them throws out of the parse, which surfaces as
 * the same error toast this exists to prevent. Their values are never
 * dereferenced on the no-update path — nothing is ever downloaded — so the
 * digest is a placeholder rather than a real hash of anything.
 */
const UPDATE_CHANNEL_YAML = `version: 0.0.0
files:
  - url: gdlauncher-0.0.0.AppImage
    sha512: ${"A".repeat(86)}==
    size: 1
path: gdlauncher-0.0.0.AppImage
sha512: ${"A".repeat(86)}==
releaseDate: '2020-01-01T00:00:00.000Z'
`

export async function startMockServer(
  opts: MockServerOptions
): Promise<MockServer> {
  const { publicKeyPem, privateKeyPem } = generateEntitlementKeypair()

  let approved = false
  const seen: string[] = []
  const failures = new Map<string, { status: number; body: string }>()

  const msTokens = () => {
    const exp = Math.floor(Date.now() / 1000) + SECONDS_IN_A_DAY

    return {
      access_token: unsignedJwt({ sub: opts.identity.oid, exp }),
      id_token: unsignedJwt({
        oid: opts.identity.oid,
        sub: opts.identity.oid,
        email: opts.identity.email,
        preferred_username: opts.identity.displayName,
        exp
      }),
      refresh_token: `mock-refresh-${opts.identity.oid}`,
      expires_in: SECONDS_IN_A_DAY
    }
  }

  const handle = async (
    req: IncomingMessage,
    res: ServerResponse
  ): Promise<void> => {
    const url = new URL(req.url ?? "/", "http://127.0.0.1")
    const route = url.pathname
    seen.push(`${req.method} ${route}`)

    const forced = failures.get(route)
    if (forced) {
      failures.delete(route)
      res.writeHead(forced.status, { "content-type": "application/json" })
      res.end(forced.body)
      return
    }

    // --- control plane -------------------------------------------------
    if (route === "/__control/approve") {
      approved = true
      json(res, 200, { approved: true })
      return
    }

    if (route.startsWith("/__control/fail")) {
      const body = JSON.parse((await readBody(req)).toString("utf8"))
      failures.set(body.route, { status: body.status, body: body.body })
      json(res, 200, { armed: body.route })
      return
    }

    if (route === "/__control/requests") {
      json(res, 200, { requests: seen })
      return
    }

    // --- update feed ---------------------------------------------------
    if (UPDATE_CHANNEL_FILE.test(route)) {
      res.writeHead(200, { "content-type": "text/yaml" })
      res.end(UPDATE_CHANNEL_YAML)
      return
    }

    // --- microsoft -----------------------------------------------------
    if (route === "/ms/consumers/oauth2/v2.0/devicecode") {
      json(res, 200, {
        user_code: "E2E-CODE",
        device_code: `mock-device-${opts.identity.oid}`,
        verification_uri: "https://example.invalid/e2e-device-login",
        expires_in: 900,
        interval: 1
      })
      return
    }

    if (route === "/ms/consumers/oauth2/v2.0/token") {
      const form = new URLSearchParams((await readBody(req)).toString("utf8"))

      if (form.get("grant_type") === "refresh_token") {
        json(res, 200, msTokens())
        return
      }

      if (!approved) {
        json(res, 400, { error: "authorization_pending" })
        return
      }

      json(res, 200, msTokens())
      return
    }

    // --- xbox ----------------------------------------------------------
    if (route === "/xbl/user/authenticate") {
      json(res, 200, { Token: "mock-xbl-token" })
      return
    }

    if (route === "/xsts/xsts/authorize") {
      json(res, 200, {
        Token: "mock-xsts-token",
        DisplayClaims: { xui: [{ uhs: "mock-user-hash" }] }
      })
      return
    }

    // --- minecraft services --------------------------------------------
    if (route === "/mc/authentication/login_with_xbox") {
      json(res, 200, {
        access_token: `mock-mc-${opts.identity.oid}`,
        expires_in: SECONDS_IN_A_DAY
      })
      return
    }

    if (route === "/mc/entitlements/mcstore") {
      json(res, 200, {
        signature: signEntitlementJwt(privateKeyPem, {
          entitlements: [{ name: "product_minecraft" }],
          // jsonwebtoken's default Validation requires `exp` to be present,
          // even though the mock signs a token that never actually expires
          // within a test run's lifetime.
          exp: Math.floor(Date.now() / 1000) + SECONDS_IN_A_DAY
        })
      })
      return
    }

    if (route === "/mc/minecraft/profile") {
      json(res, 200, {
        id: opts.identity.mcUuid,
        name: opts.identity.mcUsername,
        skins: [
          {
            id: "mock-skin-id",
            state: "ACTIVE",
            url: "https://textures.minecraft.net/texture/mock"
          }
        ]
      })
      return
    }

    // --- gdl api ---------------------------------------------------------
    if (route.startsWith("/gdl/")) {
      const gdlRoute = route.slice("/gdl".length)

      // The one hop a mock must own: enderium verifies the Microsoft id_token
      // against Microsoft's live JWKS, so a locally minted one is always
      // rejected there.
      if (gdlRoute === "/v1/auth/token") {
        json(res, 200, {
          access_token: opts.gdlToken,
          token_type: "Bearer",
          expires_at: Math.floor(Date.now() / 1000) + SECONDS_IN_A_DAY
        })
        return
      }

      if (opts.apiTestBase) {
        await forwardToApiTest(
          req,
          res,
          await readBody(req),
          opts.apiTestBase,
          `${gdlRoute}${url.search}`
        )
        return
      }

      const stub = standaloneStub(gdlRoute)
      if (stub) {
        res.writeHead(stub.status, { "content-type": stub.contentType })
        res.end(stub.body)
        return
      }

      res.writeHead(501, { "content-type": "text/plain" })
      res.end(
        `mock-idp standalone mode has no stub for ${req.method} ${gdlRoute}`
      )
      return
    }

    res.writeHead(501, { "content-type": "text/plain" })
    res.end(`mock-idp has no handler for ${req.method} ${route}`)
  }

  // A rejection out of `handle` (a malformed control-plane body, a client
  // aborting mid-request while `readBody` awaits `end`, ...) must fail only
  // the request it belongs to. Left uncaught, Node's default
  // `--unhandled-rejections=throw` would tear down the whole process, and
  // every later request the launcher makes would hit a closed port.
  const server: Server = createServer((req, res) => {
    handle(req, res).catch((err) => {
      if (res.headersSent || res.destroyed) {
        res.destroy()
        return
      }

      try {
        const message = err instanceof Error ? err.message : String(err)
        json(res, 500, { error: "mock-idp-internal-error", message })
      } catch {
        res.destroy()
      }
    })
  })

  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve))

  const { port } = server.address() as AddressInfo

  return {
    url: `http://127.0.0.1:${port}`,
    port,
    publicKeyPem,
    approve: () => {
      approved = true
    },
    failNext: (route, status, body) => failures.set(route, { status, body }),
    requests: () => [...seen],
    close: () =>
      new Promise<void>((resolve, reject) =>
        server.close((err) => (err ? reject(err) : resolve()))
      )
  }
}
