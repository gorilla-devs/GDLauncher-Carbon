import { Trans, useTransContext } from "@gd/i18n"
import { Checkbox } from "@gd/ui"
import { createSignal, Match, Show, Switch } from "solid-js"
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
      title={t(
        isPlural()
          ? "instances:_trn_confirm_world_deletion_title_plural"
          : "instances:_trn_confirm_world_deletion_title"
      )}
      width="w-120"
      preventClose={busy()}
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-4">
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
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default ConfirmWorldDeletion
