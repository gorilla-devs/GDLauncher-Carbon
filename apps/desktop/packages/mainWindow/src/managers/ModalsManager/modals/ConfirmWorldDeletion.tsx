import { Trans } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import { createSignal, Match, Show, Switch } from "solid-js"
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
  /** Everything confirming deletes, worlds and non-worlds alike. Only the
   *  bulk path sets it — a world is what *raises* this dialog, but a bulk
   *  delete removes the whole selection, so the copy below has to account for
   *  the addons that are not worlds as well as the ones that are. */
  const totalCount = () => props.data?.totalCount as number | undefined
  const isPlural = () => (worldCount() ?? 0) > 1
  /** True when the confirm button removes more than the world(s) named above. */
  const deletesMoreThanWorlds = () => (totalCount() ?? 0) > (worldCount() ?? 1)
  const onConfirm = () => props.data?.onConfirm as () => void

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} width="w-120">
      <div class="flex min-h-40 flex-col justify-between gap-6 overflow-hidden">
        <div class="flex flex-col gap-4">
          <div class="flex items-center gap-2 text-xl font-bold text-red-400">
            <div class="i-hugeicons:alert-02 h-6 w-6 shrink-0" />
            <Show
              when={isPlural()}
              fallback={
                <Trans key="instances:_trn_confirm_world_deletion_title" />
              }
            >
              <Trans key="instances:_trn_confirm_world_deletion_title_plural" />
            </Show>
          </div>
          <div class="text-lightSlate-300 flex flex-col gap-2">
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
            <Show when={deletesMoreThanWorlds()}>
              <div>
                <Trans
                  key="instances:_trn_confirm_world_deletion_body_selection"
                  options={{ count: totalCount() }}
                />
              </div>
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
              //
              // The delete runs from `finally` because it is the action the
              // user actually confirmed, while the dismissal is a preference:
              // a rejected upsert must not abort the handler, or neither the
              // delete nor the modal close would run and the click would be
              // silently dropped.
              try {
                if (dontAskAgain()) {
                  await dismissWarningMutation.mutateAsync(true)
                }
              } catch (error) {
                console.error(
                  "Failed to persist the world deletion warning dismissal:",
                  error
                )
              } finally {
                onConfirm()?.()
                modalsContext?.closeModal()
              }
            }}
          >
            <Switch
              fallback={
                <Trans key="instances:_trn_confirm_world_deletion_confirm" />
              }
            >
              <Match when={deletesMoreThanWorlds()}>
                <Trans
                  key="instances:_trn_confirm_world_deletion_confirm_selection"
                  options={{ count: totalCount() }}
                />
              </Match>
              <Match when={isPlural()}>
                <Trans key="instances:_trn_confirm_world_deletion_confirm_plural" />
              </Match>
            </Switch>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmWorldDeletion
