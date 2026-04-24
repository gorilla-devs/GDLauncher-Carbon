import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createSignal } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { FEShareInfo } from "@gd/core_module/bindings"
import { MAX_DOWNLOADS_LIMIT, validateMaxDownloads } from "@/utils/validation"

interface Props {
  share: FEShareInfo
  onUpdated: () => void
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

  const handleSave = async () => {
    const share = data()?.share
    if (!share) return

    const newTitle = editTitle().trim() || null
    const maxDownloadsValue = editMaxDownloads().trim()
    const newMaxDownloads = maxDownloadsValue
      ? parseInt(maxDownloadsValue) >= 1
        ? parseInt(maxDownloadsValue)
        : null
      : null

    try {
      await updateMutation.mutateAsync({
        shareCode: share.shareCode,
        title: newTitle,
        maxDownloads: newMaxDownloads
      })
      toast.success(t("instances:_trn_my_shares.updated"))
      data()?.onUpdated()
      modalsContext?.closeModal()
    } catch {
      toast.error(t("instances:_trn_my_shares.update_failed"))
    }
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

        <div class="flex justify-between">
          <Button type="secondary" onClick={() => modalsContext?.closeModal()}>
            <Trans key="instances:_trn_instance_share.cancel" />
          </Button>
          <Button
            type="primary"
            onClick={handleSave}
            loading={updateMutation.isPending}
          >
            <Trans key="instances:_trn_my_shares.save" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default EditShare
