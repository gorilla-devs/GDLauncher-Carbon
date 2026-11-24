import { describe, it, expect, vi, beforeEach } from "vitest"

const toastError = vi.fn()
const toastSuccess = vi.fn()
const mutation = vi.fn()

vi.mock("@gd/ui", () => ({
  toast: { error: toastError, success: toastSuccess }
}))

vi.mock("@gd/i18n", () => ({
  i18n: {
    t: (key: string, options: Record<string, unknown>) =>
      `${key}|${JSON.stringify(options)}`
  }
}))

vi.mock("./rspcClient", () => ({
  rspc: { useContext: () => ({ client: { mutation } }) },
  extractErrorDisplay: (error: Error) => error.message
}))

const { isUpdatingManagedJava, updateManagedJava } =
  await import("./managedJavaUpdates")

describe("updateManagedJava", () => {
  beforeEach(() => {
    toastError.mockClear()
    toastSuccess.mockClear()
    mutation.mockReset()
  })

  it("marks the Java as updating until it finishes, and toasts the version", async () => {
    let finish: (id: string) => void = () => {}
    mutation.mockReturnValue(new Promise((resolve) => (finish = resolve)))

    const running = updateManagedJava("z17", "17.0.21.1")
    expect(isUpdatingManagedJava("z17")).toBe(true)
    expect(isUpdatingManagedJava("z21")).toBe(false)

    finish("z17-new")
    await running
    expect(isUpdatingManagedJava("z17")).toBe(false)
    expect(mutation).toHaveBeenCalledWith(["java.updateManagedJava", "z17"])
    expect(toastSuccess).toHaveBeenCalledWith(
      'java:_trn_java_updated|{"version":"17.0.21.1"}'
    )
  })

  it("ignores a second update of a Java already updating", async () => {
    let finish: (id: string) => void = () => {}
    mutation.mockReturnValue(new Promise((resolve) => (finish = resolve)))

    const first = updateManagedJava("z17", "17.0.21.1")
    await updateManagedJava("z17", "17.0.21.1")
    finish("z17-new")
    await first

    expect(mutation).toHaveBeenCalledTimes(1)
    expect(toastSuccess).toHaveBeenCalledTimes(1)
  })

  it("toasts the error and frees the Java for another try", async () => {
    mutation.mockRejectedValue(new Error("not enough disk space"))

    await updateManagedJava("z17", "17.0.21.1")

    expect(isUpdatingManagedJava("z17")).toBe(false)
    expect(toastSuccess).not.toHaveBeenCalled()
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError.mock.calls[0][0]).toContain(
      'java:_trn_update_failed|{"error":"not enough disk space"'
    )
  })
})
