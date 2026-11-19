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

import { testOrder } from "./CustomCommands"

it("offers the instances to test a launcher-wide hook in with the one played last first", () => {
  expect(testOrder([])).toEqual([])
  const order = (instances: { id: number; last_played: string | null }[]) =>
    testOrder(instances).map((instance) => instance.id)
  expect(
    order([
      { id: 1, last_played: null },
      { id: 2, last_played: null }
    ])
  ).toEqual([1, 2])
  expect(
    order([
      { id: 1, last_played: "2026-11-01T10:00:00Z" },
      { id: 2, last_played: null },
      { id: 3, last_played: "2026-11-18T09:30:00Z" },
      { id: 4, last_played: null }
    ])
  ).toEqual([3, 1, 2, 4])
})
