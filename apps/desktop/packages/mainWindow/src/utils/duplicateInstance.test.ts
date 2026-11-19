import { beforeEach, expect, it, vi } from "vitest"

const io = vi.hoisted(() => ({
  copy: vi.fn(),
  meta: undefined as unknown,
  loading: vi.fn(),
  error: vi.fn(),
  dismiss: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  toast: { loading: io.loading, error: io.error, dismiss: io.dismiss }
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options: { name: string }) => `${key} ${options.name}`
  ]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (options: () => { meta: unknown }) => {
      io.meta = options().meta
      return { mutateAsync: io.copy }
    }
  }
}))
vi.mock("@/utils/mutationErrorToast", () => ({
  LOCAL_ERROR_TOAST: { localErrorToast: true }
}))
vi.mock("@/utils/errorToastCopyAction", () => ({
  errorToastCopyAction: (text: string) => ({ copies: text })
}))
vi.mock("@/utils/rspcErrorText", () => ({
  extractErrorDisplay: (error: Error) => error.message
}))

import { isDuplicating, useDuplicateInstance } from "./duplicateInstance"

const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
const notice = (): string => io.loading.mock.calls.at(-1)![1].id

beforeEach(() => vi.clearAllMocks())

it("shows a notice for as long as the copy is made and takes it away when the copy is there", async () => {
  let finish!: () => void
  io.copy.mockReturnValue(new Promise<void>((resolve) => (finish = resolve)))
  const duplicate = useDuplicateInstance()
  // Its failure is told by the notice, not a second time by the global one.
  expect(io.meta).toEqual({ localErrorToast: true })
  expect(isDuplicating(4)).toBe(false)
  const done = duplicate(4, "Steve's Pack")
  expect(io.copy).toHaveBeenCalledWith({
    instance: 4,
    new_name: "Steve's Pack"
  })
  expect(io.loading).toHaveBeenCalledWith(
    "instances:_trn_duplicating Steve's Pack",
    { id: expect.stringMatching(/^duplicate-4-\d+$/), duration: Infinity }
  )
  expect(isDuplicating(4)).toBe(true)
  expect(isDuplicating(5)).toBe(false)

  // A second click while it copies starts no second copy.
  await duplicate(4, "Steve's Pack")
  expect(io.copy).toHaveBeenCalledTimes(1)
  // Another instance is its own copy.
  io.copy.mockResolvedValueOnce(undefined)
  await duplicate(5, "Other")
  expect(io.copy).toHaveBeenCalledTimes(2)

  const first = io.loading.mock.calls[0][1].id
  finish()
  await done
  expect(io.dismiss).toHaveBeenCalledWith(first)
  expect(io.error).not.toHaveBeenCalled()
  expect(isDuplicating(4)).toBe(false)
})

it("turns the notice into the reason when the copy fails, and starts the next try in a new one", async () => {
  io.copy.mockRejectedValueOnce(new Error("No space left on device"))
  const duplicate = useDuplicateInstance()
  await duplicate(9, "Big")
  await flush()
  const failed = notice()
  expect(io.error).toHaveBeenCalledWith("instances:_trn_duplicate_failed Big", {
    id: failed,
    description: "No space left on device",
    action: { copies: "No space left on device" },
    duration: Infinity
  })
  expect(isDuplicating(9)).toBe(false)

  // Reused, the failure's text would stay under the spinner.
  io.copy.mockResolvedValueOnce(undefined)
  await duplicate(9, "Big")
  expect(notice()).not.toBe(failed)
  expect(io.dismiss.mock.calls[0]).toEqual([failed])
  expect(io.dismiss.mock.invocationCallOrder[0]).toBeLessThan(
    io.loading.mock.invocationCallOrder[1]
  )
})
