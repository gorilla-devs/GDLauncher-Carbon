import { describe, expect, it } from "vitest"
import { STANDALONE_STUBS, standaloneStub } from "./proxy.js"

describe("standaloneStub", () => {
  it("covers every route the launcher hits before the library renders", () => {
    // Confirmed against the call sites in gdl_account.rs, terms_and_privacy.rs,
    // api/mod.rs, and managers/mod.rs (the singular `/v1/announcement`, a
    // separate call from the plural `/v1/announcements`). A missing entry
    // surfaces as a 501 naming the route.
    expect(Object.keys(STANDALONE_STUBS).sort()).toEqual([
      "/v1/announcement",
      "/v1/announcements",
      "/v1/latest_consent_checksum",
      "/v1/privacy_statement_md",
      "/v1/record_consent",
      "/v1/terms_of_service_md",
      "/v1/users/user"
    ])
  })

  it("returns a payload shaped like GDLUser (gdl_account.rs), not a guess", () => {
    const stub = standaloneStub("/v1/users/user")
    const user = JSON.parse(stub!.body)

    expect(stub?.status).toBe(200)
    // Every GDLUser field that is neither `Option<_>` nor `#[serde(default)]`
    // must be present under its exact serde name — this is the assertion
    // that catches a renamed or missing required field, not just "some json
    // came back".
    expect(user).toMatchObject({
      email: expect.any(String),
      microsoft_oid: expect.any(String),
      display_name: expect.any(String),
      friend_code: expect.any(String),
      profile_icon_url: expect.any(String),
      is_verified: expect.any(Boolean),
      has_pending_verification: expect.any(Boolean),
      has_pending_deletion_request: expect.any(Boolean)
    })
  })

  it("returns raw text, not a json envelope, for the consent routes", () => {
    // TermsAndPrivacy::get_latest_consent_sha and ::record_consent
    // (terms_and_privacy.rs) both read the response with `.text()`; a JSON
    // body would be stored verbatim as the literal string `{"checksum":...}`.
    expect(standaloneStub("/v1/latest_consent_checksum")?.contentType).toBe(
      "text/plain"
    )
    expect(standaloneStub("/v1/latest_consent_checksum")?.body).not.toMatch(
      /^[{[]/
    )
    expect(standaloneStub("/v1/record_consent")?.contentType).toBe("text/plain")
    expect(standaloneStub("/v1/record_consent")?.body).not.toMatch(/^[{[]/)
  })

  it("returns markdown, not json, for the document routes", () => {
    expect(standaloneStub("/v1/terms_of_service_md")?.contentType).toBe(
      "text/markdown"
    )
    expect(standaloneStub("/v1/terms_of_service_md")?.body).not.toMatch(/^[{[]/)
  })

  it("is undefined for an unknown route", () => {
    expect(standaloneStub("/v1/instance-share/quota")).toBeUndefined()
  })
})
