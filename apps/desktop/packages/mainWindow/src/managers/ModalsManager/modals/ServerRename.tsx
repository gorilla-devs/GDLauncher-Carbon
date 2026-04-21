import { createSignal } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input } from "@gd/ui"
import { useTransContext } from "@gd/i18n"

const ServerRename = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [name, setName] = createSignal(props.data?.name || "")

  const updateServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.updateServer"],
    onSuccess: () => {
      modalsContext?.closeModal()
    }
  }))

  const handleSave = () => {
    const trimmed = name().trim()
    if (!trimmed) return
    updateServerMutation.mutate({
      id: props.data?.id,
      name: trimmed,
      xmx: null,
      xms: null,
      extraJavaArgs: null,
      autoRestart: null
    })
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-50"
      width="w-100"
    >
      <div class="flex h-full flex-col justify-between gap-4">
        <div class="flex flex-col gap-2">
          <label class="text-sm text-lightSlate-400">
            {t("instances:_trn_server_rename_label")}
          </label>
          <Input
            value={name()}
            onInput={(e) => setName(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleSave()
            }}
            autofocus
          />
        </div>
        <div class="flex w-full justify-between">
          <Button
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            <div class="i-hugeicons:cancel-01" />
            {t("instances:_trn_server_rename_cancel")}
          </Button>
          <Button
            type="secondary"
            onClick={handleSave}
            disabled={!name().trim() || name().trim() === props.data?.name}
          >
            <div class="i-hugeicons:tick-02" />
            {t("instances:_trn_server_rename_save")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ServerRename
