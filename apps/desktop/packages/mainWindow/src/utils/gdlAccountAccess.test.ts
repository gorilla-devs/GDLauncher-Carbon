import { expect, it } from "vitest"
import { gdlAccountAccess } from "./gdlAccountAccess"

const valid = (isEmailVerified: boolean) =>
  ({ status: "valid", value: { isEmailVerified } }) as any

it("claims nothing while the core has not answered", () => {
  expect(gdlAccountAccess(undefined)).toBe("unknown")
  expect(gdlAccountAccess(null)).toBe("unknown")
})

it.each(["unset", "skipped", "invalid"] as const)(
  "reads %s as no account",
  (status) => {
    expect(gdlAccountAccess({ status })).toBe("noAccount")
  }
)

it("tells a verified account from one that is not", () => {
  expect(gdlAccountAccess(valid(true))).toBe("ready")
  expect(gdlAccountAccess(valid(false))).toBe("unverified")
})
