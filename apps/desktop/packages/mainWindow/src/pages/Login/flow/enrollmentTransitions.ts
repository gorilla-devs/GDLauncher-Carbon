import type { FlowController } from "./types"

/** Own the entire polled Complete continuation, including its error path. */
export async function completeEnrollment(flow: FlowController, flowId: string) {
  const revision = flow.captureEnrollment()
  try {
    if (!(await flow.finalizeEnrollment(flowId, revision))) return
    if (!flow.ownsEnrollment(revision)) return
    const gdlState = await flow.checkGDLAccount()
    if (!flow.ownsEnrollment(revision)) return
    await flow.goToStep({ type: "gdl-account", gdlAccount: gdlState })
  } catch (error) {
    if (!flow.ownsEnrollment(revision)) return
    console.error("[EnrollingStep] Failed to finalize enrollment:", error)
    await flow.goToStep({
      type: "error",
      message: "Failed to save account. Please try again.",
      canRetry: true
    })
  }
}

export async function switchEnrollment(
  flow: FlowController,
  loading: (value: boolean) => void
) {
  let revision = flow.captureEnrollment()
  loading(true)
  try {
    const cancelled = flow.cancelEnrollment()
    // This operation intentionally retires its old revision. Capture the new
    // revision synchronously, before cancellation can yield to another caller.
    revision = flow.captureEnrollment()
    await cancelled
    if (!flow.ownsEnrollment(revision)) return
    const started = flow.startEnrollment("device-code")
    revision = flow.captureEnrollment()
    await started
    if (!flow.ownsEnrollment(revision)) return
    await flow.goToStep({ type: "enrolling", method: "device-code" })
  } catch (error) {
    if (!flow.ownsEnrollment(revision)) return
    throw error
  } finally {
    if (flow.ownsEnrollment(revision)) loading(false)
  }
}

export async function leaveEnrollment(flow: FlowController) {
  // Retirement is synchronous. Backend cancellation remains sequenced ahead of
  // any new begin, while the user can leave without waiting for the transport.
  void flow.cancelEnrollment()
  await flow.goToStep({ type: "auth-method" }, { direction: "backward" })
}
