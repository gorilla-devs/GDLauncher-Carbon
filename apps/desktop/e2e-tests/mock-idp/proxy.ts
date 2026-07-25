import type { IncomingMessage, ServerResponse } from "node:http"

export interface StubResponse {
  status: number
  contentType: string
  body: string
}

/**
 * The GDL routes the launcher calls between startup and the library
 * rendering, answered locally when there is no api-test to proxy to.
 *
 * Derived from the call sites in `gdl_account.rs`, `terms_and_privacy.rs`, and
 * `api/mod.rs`. `/v1/auth/token` is handled by the server itself, since its
 * response body depends on the run's provisioned token.
 */
export const STANDALONE_STUBS: Record<string, StubResponse> = {
  // Shape matches `GDLUser` (gdl_account.rs) field-for-field: every field
  // that is neither `Option<_>` nor `#[serde(default)]` is present under its
  // serde name (the struct's `rename_all = "snake_case"` is a no-op here,
  // since the Rust field names are already snake_case). The `Option` fields
  // are included as `null` rather than omitted, because the struct has no
  // `skip_serializing_if` — a real response serializes them too.
  "/v1/users/user": {
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({
      email: "e2e@e2e.invalid",
      microsoft_oid: "e2e2e2e2-e2e2-0e2e-8e2e-000000000000",
      display_name: "e2e_standalone",
      friend_code: "E2E-STANDALONE",
      profile_icon_url: "https://example.invalid/e2e-standalone-icon.png",
      has_custom_avatar: false,
      microsoft_email: "e2e@e2e.invalid",
      is_verified: true,
      has_pending_verification: false,
      has_pending_deletion_request: false,
      verification_timeout: null,
      deletion_timeout: null,
      email_change_timeout: null,
      display_name_change_timeout: null,
      verification_timeout_at: null,
      deletion_timeout_at: null,
      email_change_timeout_at: null,
      display_name_change_timeout_at: null,
      scheduled_deletion_effective_at: null
    })
  },
  // `TermsAndPrivacy::get_latest_consent_sha` (terms_and_privacy.rs) reads
  // the response with `.text()`, not `.json()` — the body is the checksum
  // string itself, not a JSON envelope around it.
  "/v1/latest_consent_checksum": {
    status: 200,
    contentType: "text/plain",
    body: "e2e-standalone-consent"
  },
  // `fetch_terms_of_service_body` reads this with `.text()` and feeds it
  // straight to the markdown renderer, so the body is raw markdown.
  "/v1/terms_of_service_md": {
    status: 200,
    contentType: "text/markdown",
    body: "# Terms of Service\n\nStandalone e2e stub.\n"
  },
  // `fetch_privacy_statement_body` reads this with `.text()` too.
  "/v1/privacy_statement_md": {
    status: 200,
    contentType: "text/markdown",
    body: "# Privacy Statement\n\nStandalone e2e stub.\n"
  },
  // `TermsAndPrivacy::record_consent` reads the response with `.text()` — the
  // accepted checksum comes back as a bare string, not `{ "recorded": true }`.
  "/v1/record_consent": {
    status: 200,
    contentType: "text/plain",
    body: "e2e-standalone-consent"
  },
  // `api/mod.rs` decodes this with `.json::<Vec<Announcement>>()`; an empty
  // array satisfies that regardless of `Announcement`'s own field shape.
  "/v1/announcements": {
    status: 200,
    contentType: "application/json",
    body: JSON.stringify([])
  }
}

export function standaloneStub(route: string): StubResponse | undefined {
  return STANDALONE_STUBS[route]
}

/**
 * Header names stripped from a forwarded request.
 *
 * `host` must name the upstream, not the mock the launcher actually dialed.
 * The rest are hop-by-hop (RFC 7230 §6.1) or otherwise scoped to the
 * launcher-to-mock connection: relaying `transfer-encoding`, `upgrade`, or
 * `keep-alive` verbatim makes `fetch` throw synchronously on the forwarded
 * call, and `te`/`trailer`/`proxy-authorization`/`proxy-authenticate` have no
 * meaning one hop further on. `content-length` is dropped and recomputed
 * below from the bytes actually sent. `accept-encoding` is deliberately not
 * in this set: `fetch` decompresses the upstream response transparently, and
 * the `content-length` this function writes back is recomputed from those
 * decoded bytes, so relaying it is already correct.
 */
const STRIPPED_HEADERS = new Set([
  "host",
  "connection",
  "keep-alive",
  "transfer-encoding",
  "upgrade",
  "te",
  "trailer",
  "proxy-authorization",
  "proxy-authenticate",
  "content-length"
])

/**
 * Replays a request against the real api-test host and streams the answer back
 * unchanged, so every GDL call except the token exchange exercises the real
 * backend with a real token.
 */
export async function forwardToApiTest(
  req: IncomingMessage,
  res: ServerResponse,
  body: Buffer,
  apiTestBase: string,
  route: string
): Promise<void> {
  const target = new URL(route, apiTestBase)
  const headers = new Headers()

  for (const [key, value] of Object.entries(req.headers)) {
    if (STRIPPED_HEADERS.has(key)) {
      continue
    }
    if (typeof value === "string") {
      headers.set(key, value)
    }
  }

  const upstream = await fetch(target, {
    method: req.method,
    headers,
    // fetch's BodyInit doesn't structurally accept a Buffer; wrapping it in
    // a Uint8Array carries the same bytes in a type fetch will accept.
    body:
      req.method === "GET" || req.method === "HEAD"
        ? undefined
        : new Uint8Array(body)
  })

  const payload = Buffer.from(await upstream.arrayBuffer())

  res.writeHead(upstream.status, {
    "content-type": upstream.headers.get("content-type") ?? "application/json",
    "content-length": payload.byteLength
  })
  res.end(payload)
}
