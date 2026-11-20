import { expect, it } from "vitest"
import { searchSidebarApplies } from "./searchSidebar"

const search = (
  share: boolean,
  direct: boolean,
  addingToInstanceOrServer: boolean
) => searchSidebarApplies({ share, direct, addingToInstanceOrServer })

it("stays for a normal search", () => {
  expect(search(false, false, false)).toBe(true)
  expect(search(false, false, true)).toBe(true)
})

it("goes for a share preview, which is a direct query too", () => {
  expect(search(true, true, false)).toBe(false)
  expect(search(true, true, true)).toBe(false)
})

it("goes for a direct lookup, unless it adds to an instance or server", () => {
  expect(search(false, true, false)).toBe(false)
  expect(search(false, true, true)).toBe(true)
})
