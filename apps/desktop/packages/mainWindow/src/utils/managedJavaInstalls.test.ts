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

const { startManagedJavaInstall, claimInstallError } =
  await import("./managedJavaInstalls")

const args = { vendor: "azul" as const, id: "build-1" }
const label = { version: "25.0.5.1", major: 25 }
// Failures are logged; the tests read the log rather than print it.
const consoleError = vi.spyOn(console, "error").mockImplementation(() => {})
const flushTimers = () => new Promise((resolve) => setTimeout(resolve, 0))

describe("startManagedJavaInstall", () => {
  beforeEach(() => {
    toastError.mockClear()
    toastSuccess.mockClear()
    mutation.mockReset()
    consoleError.mockClear()
  })

  it("resolves with the new Java id and toasts its name, as its row shows it", async () => {
    mutation.mockResolvedValue("java-id")

    await expect(startManagedJavaInstall(args, label)).resolves.toBe("java-id")
    expect(mutation).toHaveBeenCalledWith(["java.setupManagedJava", args])
    expect(toastSuccess).toHaveBeenCalledWith(
      'java:_trn_java_installed|{"vendor":"Azul Zulu","version":"25.0.5.1"}'
    )
  })

  it("toasts what failed when no dialog claims it, and logs the reason", async () => {
    mutation.mockRejectedValue(new Error("connection reset"))

    await expect(startManagedJavaInstall(args, label)).rejects.toThrow(
      "connection reset"
    )
    await flushTimers()

    expect(toastError).toHaveBeenCalledTimes(1)
    // The backend's reason is technical: the log keeps it, not the toast.
    expect(toastError).toHaveBeenCalledWith(
      'java:_trn_download_failed|{"major":25}'
    )
    expect(consoleError.mock.calls.flat()).toContain("connection reset")
  })

  it("stays quiet when the open dialog claims the error", async () => {
    mutation.mockRejectedValue(new Error("connection reset"))

    try {
      await startManagedJavaInstall(args, label)
    } catch {
      claimInstallError(args.id)
    }
    await flushTimers()

    expect(toastError).not.toHaveBeenCalled()
  })

  it("doesn't let an old claim silence a later failure", async () => {
    claimInstallError(args.id)
    mutation.mockRejectedValue(new Error("disk full"))

    await expect(startManagedJavaInstall(args, label)).rejects.toThrow(
      "disk full"
    )
    await flushTimers()

    expect(toastError).toHaveBeenCalledTimes(1)
  })
})
