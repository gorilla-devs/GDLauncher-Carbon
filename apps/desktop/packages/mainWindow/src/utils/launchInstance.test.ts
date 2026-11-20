import { beforeEach, expect, it, vi } from "vitest"

const io = vi.hoisted(() => ({
  mutate: vi.fn(),
  pending: false,
  key: undefined as unknown
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (options: () => { mutationKey: unknown }) => {
      io.key = options().mutationKey
      return {
        mutate: io.mutate,
        get isPending() {
          return io.pending
        }
      }
    }
  }
}))

import { useLaunchInstance } from "./launchInstance"

beforeEach(() => {
  vi.clearAllMocks()
  io.pending = false
})

it("asks the core to start the game and leaves the account and memory checks to it", () => {
  const { launch, launching } = useLaunchInstance()
  expect(io.key).toEqual(["instance.launchInstance"])
  expect(launching()).toBe(false)
  launch(4)
  // Neither check is skipped: only its own dialog's "Launch anyway" does.
  expect(io.mutate).toHaveBeenCalledWith({ id: 4, skipMemoryCheck: false })
})

it("shows as launching until the core answers, and ignores a press in between", () => {
  const { launch, launching } = useLaunchInstance()
  launch(4)
  io.pending = true
  expect(launching()).toBe(true)
  launch(4)
  expect(io.mutate).toHaveBeenCalledTimes(1)
  io.pending = false
  expect(launching()).toBe(false)
  launch(4)
  expect(io.mutate).toHaveBeenCalledTimes(2)
})
