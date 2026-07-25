import { describe, expect, it } from "vitest"
import { buildCoreModuleArgs } from "../coreArgs.js"

describe("buildCoreModuleArgs", () => {
  it("always passes the runtime path first", () => {
    expect(buildCoreModuleArgs({ runtimePath: "/data" })).toEqual([
      "--runtime_path",
      "/data"
    ])
  })

  it("appends the base api override when present", () => {
    expect(
      buildCoreModuleArgs({ runtimePath: "/data", baseApi: "http://api" })
    ).toEqual(["--runtime_path", "/data", "--base_api", "http://api"])
  })

  it("appends both e2e overrides when present", () => {
    expect(
      buildCoreModuleArgs({
        runtimePath: "/data",
        e2eAuthBase: "http://127.0.0.1:1234",
        e2eEntitlementKey: "/tmp/key.pem"
      })
    ).toEqual([
      "--runtime_path",
      "/data",
      "--e2e_auth_base",
      "http://127.0.0.1:1234",
      "--e2e_entitlement_key",
      "/tmp/key.pem"
    ])
  })

  it("omits an override that was passed without a value", () => {
    // validateArgument yields { value: null } for a bare flag. Forwarding the
    // flag with nothing after it would make the core module swallow whichever
    // argument came next as its value.
    expect(
      buildCoreModuleArgs({
        runtimePath: "/data",
        baseApi: null,
        e2eAuthBase: null,
        e2eEntitlementKey: null
      })
    ).toEqual(["--runtime_path", "/data"])
  })
})
