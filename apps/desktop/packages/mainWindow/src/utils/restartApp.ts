import { createSignal } from "solid-js"

/**
 * Settings > Restart app, one restart at a time. A restart can take a while
 * (a local server saving, with the shutdown prompt on screen), and a second
 * click meanwhile would be refused as a second runtime operation and shown
 * as a failed restart while the first one carries on. So the button is
 * disabled while `restarting()`, and a click then does nothing. `failed`
 * is told only about a restart that failed outright; keeping GDLauncher
 * open simply enables the button again.
 */
export function createRestart(
  relaunch: () => Promise<unknown>,
  failed: (error: unknown) => void
) {
  const [restarting, setRestarting] = createSignal(false)
  const restart = async () => {
    if (restarting()) return
    setRestarting(true)
    try {
      await relaunch()
    } catch (error) {
      failed(error)
    } finally {
      setRestarting(false)
    }
  }
  return { restarting, restart }
}
