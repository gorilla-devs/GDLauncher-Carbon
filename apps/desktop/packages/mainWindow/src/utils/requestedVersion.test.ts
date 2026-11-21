import { expect, it } from "vitest"
import { validatedRequestedVersion } from "./requestedVersion"
it("navigation version ownership must be validated before exact install", () => {
  expect(
    validatedRequestedVersion("chosen", "owner", undefined)
  ).toBeUndefined()
  expect(
    validatedRequestedVersion("chosen", "owner", {
      id: "chosen",
      project_id: "owner"
    })
  ).toBe("chosen")
  expect(() =>
    validatedRequestedVersion("chosen", "owner", {
      id: "chosen",
      project_id: "foreign"
    })
  ).toThrow("different project")
  expect(() =>
    validatedRequestedVersion("chosen", "owner", {
      id: "other",
      project_id: "owner"
    })
  ).toThrow()
  expect(
    validatedRequestedVersion(undefined, "owner", undefined)
  ).toBeUndefined()
})
