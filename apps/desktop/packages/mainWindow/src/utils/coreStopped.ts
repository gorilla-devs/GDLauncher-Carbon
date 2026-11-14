import { createSignal } from "solid-js"

/** How the core ended: an exit code, or the signal that ended it. */
export interface CoreExit {
  code: number | null
  signal: string | null
}

const [coreStopped, setCoreStopped] = createSignal<CoreExit>()
/**
 * Set once main reports that the background service stopped by itself.
 * Nothing answers from then on, so request failures are not reported.
 */
export { coreStopped }

/** Follows main's report, including one from before this page loaded. */
export function watchCoreStopped(): () => void {
  const stop = window.onCoreExited?.((exit) => setCoreStopped(exit))
  void window
    .coreExit?.()
    .then((exit) => {
      if (exit) setCoreStopped(exit)
    })
    .catch((error) =>
      console.warn("Could not ask whether the core runs", error)
    )
  return () => stop?.()
}

/** For support: Windows reports a crash as a large code, read in hex. */
export function coreExitCode(exit: CoreExit): string | undefined {
  if (exit.code === null) return undefined
  return exit.code > 0xffff
    ? `0x${exit.code.toString(16).toUpperCase()}`
    : String(exit.code)
}

/** Tests only. */
export function resetCoreStopped() {
  setCoreStopped(undefined)
}
