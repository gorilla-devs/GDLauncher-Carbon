import { describe, test, expect } from "vitest"
import { NETWORK_ERROR_RE } from "../utils/errorClassification"

// The classifier runs on `error.message`, so the browser's transport failure
// arrives as the bare string "Failed to fetch". The backend prefixes many
// unrelated errors with "Failed to fetch <resource>", and those must not be
// reported to the user as connectivity problems.
describe("NETWORK_ERROR_RE", () => {
  test.each([
    ["Failed to fetch", "browser transport failure"],
    ["TypeError: Failed to fetch", "stringified transport failure"],
    ["error sending request for url (http://127.0.0.1:1025/)", "reqwest"],
    ["Connection refused (os error 111)", "refused connection"],
    ["failed to make network request", "backend transport wrapper"]
  ])("treats %j as a network error (%s)", (message) => {
    expect(NETWORK_ERROR_RE.test(message)).toBe(true)
  })

  test.each([
    ["Failed to fetch version manifest", "missing resource"],
    ["Failed to fetch server pack file info: 404 Not Found", "http status"],
    [
      "Failed to fetch minecraft version: unexpected end of JSON",
      "parse error"
    ],
    ["Failed to fetch latest consent sha: 500", "server error"]
  ])("does not treat %j as a network error (%s)", (message) => {
    expect(NETWORK_ERROR_RE.test(message)).toBe(false)
  })
})
