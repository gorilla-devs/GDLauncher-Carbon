import { Show, createSignal, onCleanup } from "solid-js"
import { Portal } from "solid-js/web"
import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import adSize, { adLayoutMounted } from "@/utils/adhelper"
import {
  coreExitCode,
  coreStopped,
  watchCoreStopped
} from "@/utils/coreStopped"

/**
 * Shown over the launcher when its background service stopped by itself
 * (a crash, the system ending it): nothing works until a restart, and the
 * failing requests would otherwise read as a network problem. Main decides
 * when that is the case and never reports a stop it asked for.
 *
 * It renders as the shutdown prompt does: into the body, above everything
 * but the ad column.
 */
export default function CoreStopped() {
  onCleanup(watchCoreStopped())
  const [restarting, setRestarting] = createSignal(false)

  const restart = async () => {
    setRestarting(true)
    try {
      await window.relaunch()
    } catch (error) {
      console.error("Could not restart GDLauncher", error)
    }
    // Still here: the restart did not happen.
    setRestarting(false)
  }

  return (
    <Show when={coreStopped()}>
      {(exit) => {
        // For support: the signal that ended it, or its exit code.
        const code = () => coreExitCode(exit())
        const paddingRight = () =>
          adLayoutMounted() ? `${adSize.width + 32}px` : undefined
        return (
          <Portal>
            <div
              class="fixed inset-0 z-[10003] flex flex-col items-center justify-center bg-black bg-opacity-65 p-8 backdrop-blur-sm"
              style={{ "padding-right": paddingRight() }}
              data-testid="core-stopped"
            >
              <div class="max-w-2xl text-center">
                <div class="text-2xl">
                  <Trans key="app:_trn_core_stopped.title" />
                </div>
                <p class="mt-4 text-base">
                  <Trans key="app:_trn_core_stopped.body" />
                </p>
                <Show when={exit().signal ?? code()}>
                  <p
                    class="text-lightSlate-600 mt-2 text-sm"
                    data-testid="core-stopped-detail"
                  >
                    <Show when={!exit().signal} fallback={exit().signal}>
                      <Trans
                        key="app:_trn_core_stopped.exit_code"
                        options={{ code: code() }}
                      />
                    </Show>
                  </p>
                </Show>
                <div class="mt-6 flex justify-center gap-4">
                  <Button
                    type="primary"
                    disabled={restarting()}
                    data-testid="core-stopped-restart"
                    onClick={() => void restart()}
                  >
                    <Trans key="settings:_trn_restart_app" />
                  </Button>
                  <Button
                    type="secondary"
                    data-testid="core-stopped-quit"
                    onClick={() => void window.closeWindow()}
                  >
                    <Trans key="window:_trn_window_close_quit_app" />
                  </Button>
                </div>
              </div>
            </div>
          </Portal>
        )
      }}
    </Show>
  )
}
