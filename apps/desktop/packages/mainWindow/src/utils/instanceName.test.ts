import { expect, it } from "vitest"
import { renameTo } from "./instanceName"

it("asks for a rename only for a name that is not blank and not the one it has", () => {
  expect(renameTo("Skyblock 2", "Skyblock")).toBe("Skyblock 2")
  expect(renameTo("  Skyblock 2 \n", "Skyblock")).toBe("Skyblock 2")
  expect(renameTo("", "Skyblock")).toBeNull()
  expect(renameTo("   ", "Skyblock")).toBeNull()
  expect(renameTo("Skyblock", "Skyblock")).toBeNull()
  expect(renameTo(" Skyblock ", "Skyblock")).toBeNull()
  expect(renameTo("skyblock", "Skyblock")).toBe("skyblock")
  expect(renameTo("New", undefined)).toBe("New")
})
