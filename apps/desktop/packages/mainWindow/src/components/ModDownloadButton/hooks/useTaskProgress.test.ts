import { beforeEach, expect, it, vi } from "vitest"
const state = vi.hoisted(() => ({
  query: vi.fn(),
  error: vi.fn(),
  success: vi.fn(),
  cleanup: [] as (() => void)[]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: { useContext: () => ({ client: { query: state.query } }) }
}))
vi.mock("@gd/ui", () => ({
  toast: { error: state.error, success: state.success }
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key],
  i18n: { t: (key: string) => key }
}))
vi.mock("solid-js", () => ({
  createSignal: (value: unknown) => [
    () => value,
    (next: unknown) => {
      value = next
    }
  ],
  createEffect: (effect: () => void) => effect(),
  onCleanup: (cleanup: () => void) => state.cleanup.push(cleanup)
}))
import { useTaskProgress } from "./useTaskProgress"
beforeEach(() => {
  vi.clearAllMocks()
  state.cleanup = []
})
it.each(["Failed", "Dismissed", "Unavailable", "Succeeded"] as const)(
  "dropdown observes %s only for its captured accepted ID",
  async (type) => {
    state.query.mockResolvedValue({
      type,
      value: {
        progress: {
          type: "Failed",
          value: { cause: [{ display: "Required dependency failed" }] }
        }
      }
    })
    const ids = new Map([[1, 73]])
    const clear = vi.fn((id: number) => {
      ids.delete(id)
    })
    useTaskProgress(() => ids, clear, { title: "Pack", type: "datapack" })
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(state.query).toHaveBeenCalledWith(["vtask.getTaskOutcome", 73])
    expect(clear).toHaveBeenCalledWith(1)
    if (type === "Succeeded") expect(state.success).toHaveBeenCalledOnce()
    else expect(state.success).not.toHaveBeenCalled()
    if (type === "Failed" || type === "Unavailable")
      expect(state.error).toHaveBeenCalledOnce()
    else expect(state.error).not.toHaveBeenCalled()
  }
)
it("unmounting before an old read finishes neither acknowledges nor reports success", async () => {
  state.query.mockImplementation(async () => {
    state.cleanup.forEach((cleanup) => cleanup())
    return { type: "Succeeded" }
  })
  const clear = vi.fn()
  useTaskProgress(() => new Map([[1, 73]]), clear)
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(clear).not.toHaveBeenCalled()
  expect(state.success).not.toHaveBeenCalled()
})
