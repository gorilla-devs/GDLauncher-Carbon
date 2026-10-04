import { Trans, useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import { Match, Switch } from "solid-js"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"

export interface RecoveredServerProcessData {
  id: number
  name: string
  /** Only the actions that need the user's confirmation open this dialog. */
  action: "forceStop" | "clearRecord"
  processId: number | null
}

/**
 * Confirms what a server left behind by an earlier session of the launcher
 * needs: ending its JVM at once on Windows, where it cannot be asked to save
 * first, or clearing a record the core cannot verify. Errors reach the user
 * through the global toast and leave the dialog open to try again.
 */
function RecoveredServerProcess(props: ModalProps) {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const data = () => props.data as RecoveredServerProcessData

  const stopMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopRecoveredServer"]
  }))
  const clearMutation = rspc.createMutation(() => ({
    mutationKey: ["server.clearServerMarker"]
  }))
  const pending = () => stopMutation.isPending || clearMutation.isPending

  const confirm = async () => {
    if (pending()) return
    try {
      if (data().action === "forceStop") {
        await stopMutation.mutateAsync(data().id)
      } else {
        await clearMutation.mutateAsync(data().id)
      }
      modalsContext?.closeModal()
    } catch {
      // Shown by the global error toast; the dialog stays for a retry.
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={
        data().action === "forceStop"
          ? t("modals:_trn_server_force_stop_title")
          : t("modals:_trn_server_clear_record_title")
      }
      width="w-130"
    >
      <div
        class="flex flex-col gap-5"
        data-testid="server-recovered-dialog"
        data-action={data().action}
      >
        <div class="flex items-center gap-2 text-lg font-bold text-yellow-400">
          <div class="i-hugeicons:alert-02 h-6 w-6 shrink-0" />
          <span>{data().name}</span>
        </div>
        <p class="text-lightSlate-300 m-0 text-sm">
          <Switch>
            <Match when={data().action === "forceStop"}>
              <Trans
                key="instances:_trn_server_force_stop_body"
                options={{ pid: data().processId ?? "" }}
              />
            </Match>
            <Match when={data().processId != null}>
              <Trans
                key="instances:_trn_server_clear_record_body"
                options={{ pid: data().processId }}
              />
            </Match>
            <Match when={true}>
              <Trans key="instances:_trn_server_clear_record_body_unreadable" />
            </Match>
          </Switch>
        </p>
        <div class="flex w-full justify-between pt-2">
          <Button
            onClick={() => modalsContext?.closeModal()}
            disabled={pending()}
          >
            <div class="i-hugeicons:cancel-01" />
            {t("instances:_trn_instance_confirm_deletion.cancel")}
          </Button>
          <Button
            variant="red"
            loading={pending()}
            disabled={pending()}
            data-testid="server-recovered-confirm"
            onClick={confirm}
          >
            <Switch>
              <Match when={data().action === "forceStop"}>
                <div class="i-hugeicons:stop" />
                <Trans key="instances:_trn_server_action_force_stop" />
              </Match>
              <Match when={true}>
                <div class="i-hugeicons:eraser" />
                <Trans key="instances:_trn_server_action_clear_record" />
              </Match>
            </Switch>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default RecoveredServerProcess
