import { Trans, useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import { Show, createEffect } from "solid-js"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"
import { stopClock } from "@/utils/serverStop"

export interface ForceStopServerData {
  id: number
  name: string
}

/**
 * Confirms ending a server that is still shutting down. The launcher never
 * does that by itself, so the dialog shows how long the stop has taken and
 * when the server last printed something: what tells a server that is still
 * saving from one that is stuck. It closes by itself once the server stopped.
 */
function ForceStopServer(props: ModalProps) {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const data = () => props.data as ForceStopServerData

  const progress = rspc.createQuery(() => ({
    queryKey: ["server.getServerStopProgress", data().id],
    refetchInterval: 1000,
    gcTime: 0
  }))
  const killMutation = rspc.createMutation(() => ({
    mutationKey: ["server.killServer"]
  }))

  createEffect(() => {
    // Null: the server is no longer stopping, so there is nothing to force.
    if (progress.isSuccess && progress.data === null && !killMutation.isPending)
      modalsContext?.closeModal()
  })

  const confirm = async () => {
    if (killMutation.isPending) return
    try {
      await killMutation.mutateAsync(data().id)
      modalsContext?.closeModal()
    } catch {
      // Shown by the global error toast; the dialog stays for a retry.
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={t("modals:_trn_server_force_stop_title")}
      width="w-130"
    >
      <div class="flex flex-col gap-5" data-testid="server-force-stop-dialog">
        <div class="flex items-center gap-2 text-lg font-bold text-yellow-400">
          <div class="i-hugeicons:alert-02 h-6 w-6 shrink-0" />
          <span class="truncate">{data().name}</span>
        </div>
        <p class="text-lightSlate-300 m-0 text-sm">
          <Trans key="instances:_trn_server_force_stop_running_body" />
        </p>
        <Show when={progress.data}>
          {(current) => (
            <div
              class="bg-darkSlate-900 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 rounded-lg px-4 py-3 text-sm"
              data-testid="server-force-stop-progress"
            >
              <span class="text-lightSlate-600">
                <Trans key="instances:_trn_server_stop_progress_elapsed" />
              </span>
              <span class="text-lightSlate-100 tabular-nums">
                {stopClock(current().stoppingSeconds)}
              </span>
              <span class="text-lightSlate-600">
                <Trans key="instances:_trn_server_stop_progress_last_output" />
              </span>
              <span class="text-lightSlate-100 min-w-0">
                <Show
                  when={current().lastOutputSeconds != null}
                  fallback={
                    <Trans key="instances:_trn_server_stop_progress_no_output" />
                  }
                >
                  <span class="tabular-nums">
                    <Trans
                      key="instances:_trn_server_stop_progress_ago"
                      options={{
                        time: stopClock(current().lastOutputSeconds ?? 0)
                      }}
                    />
                  </span>
                  <Show when={current().lastOutput}>
                    <code class="text-lightSlate-400 mt-1 block truncate font-mono text-xs">
                      {current().lastOutput}
                    </code>
                  </Show>
                </Show>
              </span>
            </div>
          )}
        </Show>
        <div class="flex w-full justify-between pt-2">
          <Button
            onClick={() => modalsContext?.closeModal()}
            disabled={killMutation.isPending}
            data-testid="server-force-stop-keep-waiting"
          >
            <Trans key="instances:_trn_server_force_stop_keep_waiting" />
          </Button>
          <Button
            type="secondary"
            textColor="!text-red-400"
            loading={killMutation.isPending}
            disabled={killMutation.isPending}
            data-testid="server-force-stop-confirm"
            onClick={confirm}
          >
            <div class="i-hugeicons:stop" />
            <Trans key="instances:_trn_server_action_force_stop" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ForceStopServer
