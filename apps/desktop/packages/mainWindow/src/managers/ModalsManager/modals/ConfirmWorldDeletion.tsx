import { Trans, useTransContext } from "@gd/i18n"
import { Checkbox } from "@gd/ui"
import { createSignal } from "solid-js"
import ModalLayout from "../ModalLayout"
import { ModalProps } from ".."
import ConfirmActions, { createConfirm } from "../ConfirmActions"
import { rspc } from "@/utils/rspcClient"

function ConfirmWorldDeletion(props: ModalProps) {
  const [t] = useTransContext()
  const [dontAskAgain, setDontAskAgain] = createSignal(false)

  const dismissWarningMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setWorldDeletionWarningDismissed"]
  }))

  const worldName = () => props.data?.worldName as string
  const onConfirm = () => props.data?.onConfirm as () => void

  // With "Deletion through Recycle Bin" on the world can be taken back out,
  // so the dialog must not say it is gone for good.
  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const toRecycleBin = () => settings.data?.deletionThroughRecycleBin === true

  const { busy, confirm, cancel } = createConfirm(() => ({
    onConfirm: async () => {
      // Persist the dismissal before running the delete, so a fast
      // follow-up delete cannot land before the upsert is durable, as
      // `InsufficientMemory` orders its own dismissal.
      //
      // A rejected upsert must not stop the delete: it is the action the
      // user confirmed, while the dismissal is only a preference.
      try {
        if (dontAskAgain()) {
          await dismissWarningMutation.mutateAsync(true)
        }
      } catch (error) {
        console.error(
          "Failed to persist the world deletion warning dismissal:",
          error
        )
      }
      onConfirm()?.()
    }
  }))

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={t("instances:_trn_confirm_world_deletion_title")}
      width="w-120"
      preventClose={busy()}
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-4">
          <div class="text-lightSlate-300 flex flex-col gap-2">
            {t(
              toRecycleBin()
                ? "instances:_trn_confirm_world_deletion_body_recycle_bin"
                : "instances:_trn_confirm_world_deletion_body",
              {
                name: worldName(),
                interpolation: { escapeValue: false }
              }
            )}
          </div>
          <Checkbox
            checked={dontAskAgain()}
            disabled={busy()}
            onChange={(checked) => setDontAskAgain(checked)}
          >
            <span
              class="text-lightSlate-300 text-sm"
              data-testid="world-deletion-dont-ask"
            >
              <Trans key="instances:_trn_confirm_world_deletion_dont_ask" />
            </span>
          </Checkbox>
        </div>

        <ConfirmActions
          busy={busy()}
          confirmTestId="world-deletion-confirm"
          onConfirm={confirm}
          onCancel={cancel}
        >
          <Trans key="instances:_trn_confirm_world_deletion_confirm" />
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default ConfirmWorldDeletion
