import { Trans, useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import { For, Show } from "solid-js"
import ModalLayout from "../ModalLayout"
import { DialogFooter } from "../DialogFooter"
import { TextLink } from "@/components/TextLink"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"
import { deletionTarget } from "@/utils/deletionTarget"
import {
  AddonToDelete,
  addonDeleteLabelKey,
  listedAddonNames
} from "@/utils/addonDeletion"
import { DeletionNoteBox, DeletionTargetLine } from "./DeletionNote"

export interface ConfirmAddonDeletionData {
  addons: AddonToDelete[]
  onConfirm: () => void
}

/**
 * Asks before deleting a selection of addons, or one file that cannot be
 * downloaded again. It names the files GDLauncher cannot get back and says
 * whether they go to the Recycle Bin.
 */
function ConfirmAddonDeletion(props: ModalProps) {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const data = () => props.data as ConfirmAddonDeletionData
  const count = () => data().addons.length
  const several = () => count() > 1
  const own = () =>
    data()
      .addons.filter((addon) => !addon.downloadable)
      .map((addon) => addon.name)
  const names = () => listedAddonNames(own())

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const target = () => deletionTarget(settings.data?.deletionThroughRecycleBin)

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={
        several()
          ? t("instances:_trn_confirm_addon_deletion_title_plural", {
              count: count()
            })
          : t("instances:_trn_confirm_addon_deletion_title")
      }
      height="h-auto"
      width="w-120"
    >
      <div class="flex flex-col gap-4" data-testid="addon-deletion-dialog">
        <Show when={!several()}>
          <div class="truncate text-lg font-bold">{data().addons[0]?.name}</div>
        </Show>
        <Show when={own().length > 0}>
          <div class="text-lightSlate-300 flex flex-col gap-2 text-sm">
            <Show
              when={several()}
              fallback={
                <Trans key="instances:_trn_confirm_addon_deletion_not_downloadable" />
              }
            >
              <Trans
                key="instances:_trn_confirm_addon_deletion_not_downloadable_plural"
                options={{ count: own().length }}
              />
              <div
                class="bg-darkSlate-700 text-lightSlate-200 rounded-lg px-3 py-2"
                data-testid="addon-deletion-own-files"
              >
                <For each={names().listed}>
                  {(name) => <div class="truncate py-0.5">{name}</div>}
                </For>
                <Show when={names().more > 0}>
                  <div class="text-lightSlate-500 py-0.5">
                    <Trans
                      key="instances:_trn_confirm_addon_deletion_more"
                      options={{ count: names().more }}
                    />
                  </div>
                </Show>
              </div>
            </Show>
          </div>
        </Show>
        <Show when={target()}>
          <DeletionNoteBox testId="addon-deletion-note">
            <DeletionTargetLine
              target={target()}
              recycleBin={
                <Trans
                  key={
                    several()
                      ? "instances:_trn_addons_delete_to_recycle_bin"
                      : "instances:_trn_addon_delete_to_recycle_bin"
                  }
                />
              }
              permanent={
                <Trans
                  key={
                    several()
                      ? "instances:_trn_addons_delete_permanent"
                      : "instances:_trn_addon_delete_permanent"
                  }
                />
              }
            />
          </DeletionNoteBox>
        </Show>
        <DialogFooter class="pt-2">
          <TextLink
            data-testid="addon-deletion-cancel"
            onClick={() => modalsContext?.closeModal()}
          >
            <Trans key="general:_trn_cancel" />
          </TextLink>
          <Button
            type="secondary"
            textColor={target() === "permanent" ? "!text-red-400" : undefined}
            data-testid="addon-deletion-confirm"
            onClick={() => {
              modalsContext?.closeModal()
              data().onConfirm()
            }}
          >
            <div class="i-hugeicons:delete-02" />
            <Trans
              key={addonDeleteLabelKey(count(), target())}
              options={{ count: count() }}
            />
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default ConfirmAddonDeletion
