import { expect, it, vi } from "vitest"

vi.mock("@gd/i18n", () => ({ Trans: () => null, useTransContext: () => [] }))
vi.mock("@gd/ui", () => ({}))
vi.mock("@/utils/rspcClient", () => ({ rspc: {} }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({})
}))
vi.mock("@/components/HookTest", () => ({ default: () => null }))
vi.mock("./components/PageTitle", () => ({ default: () => null }))
vi.mock("./components/Row", () => ({ default: () => null }))
vi.mock("./components/Title", () => ({ default: () => null }))
vi.mock("./components/RowsContainer", () => ({ default: () => null }))

import { defaultTestInstance } from "./CustomCommands"

it("tests a launcher-wide hook in the instance played last, or else the first", () => {
  expect(defaultTestInstance([])).toBeUndefined()
  expect(
    defaultTestInstance([
      { id: 1, last_played: null },
      { id: 2, last_played: null }
    ])
  ).toBe(1)
  expect(
    defaultTestInstance([
      { id: 1, last_played: "2026-11-01T10:00:00Z" },
      { id: 2, last_played: null },
      { id: 3, last_played: "2026-11-18T09:30:00Z" }
    ])
  ).toBe(3)
})
