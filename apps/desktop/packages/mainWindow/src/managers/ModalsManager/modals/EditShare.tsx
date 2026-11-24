import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { CancelLink, DialogFooter } from "../DialogFooter"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createSignal } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { FEShareInfo } from "@gd/core_module/bindings"
import { MAX_DOWNLOADS_LIMIT, validateMaxDownloads } from "@/utils/validation"
import { shareChanges, shareChangesApplied } from "@/utils/shareEdit"

interface Props {
  share: FEShareInfo
  /**
   * Reads the list again and answers with the share as it is listed now, or
   * with nothing when that could not be read.
   */
  onUpdated: () => Promise<FEShareInfo | undefined>
}

const EditShare = (props: ModalProps) => {
  const data: () => Props = () => props.data
  const [t] = useTransContext()
  const modalsContext = useModal()

  const [editTitle, setEditTitle] = createSignal(data()?.share.title || "")
  const [editMaxDownloads, setEditMaxDownloads] = createSignal(
    data()?.share.maxDownloads?.toString() || ""
  )

  const updateMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateShare"]
  }))

  const [saving, setSaving] = createSignal(false)

  const handleSave = async () => {
    const share = data()?.share
    if (!share || saving()) return

    const changes = shareChanges(share, {
      title: editTitle(),
      maxDownloads: editMaxDownloads()
    })
    if (Object.keys(changes).length === 0) {
      modalsContext?.closeModal()
      return
    }

    setSaving(true)
    try {
      await updateMutation.mutateAsync({
        shareCode: share.shareCode,
        ...changes
      })
    } catch {
      toast.error(t("instances:_trn_my_shares.update_failed"))
      setSaving(false)
      return
    }

    // The server answers the same whether or not it took the change, so what
    // it lists afterwards decides the notice. When that cannot be read, the
    // list says so itself and no notice is shown.
    const listed = await data()
      ?.onUpdated()
      ?.catch(() => undefined)
    setSaving(false)
    if (listed && !shareChangesApplied(listed, changes)) {
      toast.error(t("instances:_trn_my_shares.update_failed"))
      return
    }
    if (listed) toast.success(t("instances:_trn_my_shares.updated"))
    modalsContext?.closeModal()
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-100 flex flex-col gap-4">
        <div>
          <label class="text-lightSlate-400 mb-1 block text-sm">
            <Trans key="instances:_trn_my_shares.edit_title_label" />
          </label>
          <Input
            value={editTitle()}
            onInput={(e) => setEditTitle(e.currentTarget.value)}
            placeholder={t("instances:_trn_my_shares.name")}
            inputColor="bg-darkSlate-800"
            class="w-full"
          />
        </div>

        <div>
          <label class="text-lightSlate-400 mb-1 block text-sm">
            <Trans key="instances:_trn_my_shares.edit_max_downloads_label" />
          </label>
          <Input
            type="number"
            min="1"
            max={MAX_DOWNLOADS_LIMIT}
            value={editMaxDownloads()}
            onInput={(e) => {
              const validated = validateMaxDownloads(e.currentTarget.value)
              setEditMaxDownloads(validated)
              e.currentTarget.value = validated
            }}
            placeholder={t(
              "instances:_trn_my_shares.edit_max_downloads_placeholder"
            )}
            inputColor="bg-darkSlate-800"
            class="w-full"
          />
        </div>

        <DialogFooter>
          <CancelLink onClick={() => modalsContext?.closeModal()} />
          <Button type="primary" onClick={handleSave} loading={saving()}>
            <Trans key="instances:_trn_my_shares.save" />
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default EditShare
