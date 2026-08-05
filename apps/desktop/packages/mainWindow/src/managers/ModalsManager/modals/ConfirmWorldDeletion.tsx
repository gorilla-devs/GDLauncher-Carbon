import { Trans } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import { createSignal, Show } from "solid-js"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"

function ConfirmWorldDeletion(props: ModalProps) {
  const modalsContext = useModal()
  const [dontAskAgain, setDontAskAgain] = createSignal(false)

  const dismissWarningMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setWorldDeletionWarningDismissed"]
  }))

  const worldName = () => props.data?.worldName as string
  const worldCount = () => props.data?.worldCount as number | undefined
  const isPlural = () => (worldCount() ?? 0) > 1
  const onConfirm = () => props.data?.onConfirm as () => void

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} width="w-120">
      <div class="flex min-h-40 flex-col justify-between gap-6 overflow-hidden">
        <div class="flex flex-col gap-4">
          <div class="flex items-center gap-2 text-xl font-bold text-red-400">
            <div class="i-hugeicons:alert-02 h-6 w-6 shrink-0" />
            <Trans key="instances:_trn_confirm_world_deletion_title" />
          </div>
          <div class="text-lightSlate-300">
            <Show
              when={isPlural()}
              fallback={
                <Trans
                  key="instances:_trn_confirm_world_deletion_body"
                  options={{ name: worldName() }}
                />
              }
            >
              <Trans
                key="instances:_trn_confirm_world_deletion_body_plural"
                options={{ count: worldCount() }}
              />
            </Show>
          </div>
          <Checkbox
            checked={dontAskAgain()}
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

        <div class="flex w-full justify-between">
          <Button
            type="secondary"
            size="large"
            onClick={() => modalsContext?.closeModal()}
          >
            <Trans key="instances:_trn_confirm_world_deletion_cancel" />
          </Button>
          <Button
            type="primary"
            size="large"
            data-testid="world-deletion-confirm"
            onClick={async () => {
              // Persist the dismissal before running the delete, so a fast
              // follow-up delete cannot land before the upsert is durable —
              // same ordering `InsufficientMemory` uses for its own dismissal.
              if (dontAskAgain()) {
                await dismissWarningMutation.mutateAsync(true)
              }
              onConfirm()?.()
              modalsContext?.closeModal()
            }}
          >
            <Trans key="instances:_trn_confirm_world_deletion_confirm" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmWorldDeletion
