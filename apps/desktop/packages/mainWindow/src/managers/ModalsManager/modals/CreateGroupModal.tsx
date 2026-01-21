import { createSignal, createEffect } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { Button, Input } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useModal, ModalProps } from "@/managers/ModalsManager"
import ModalLayout from "../ModalLayout"

const CreateGroupModal = (props: ModalProps) => {
  const [t] = useTransContext()
  const modals = useModal()

  const [groupName, setGroupName] = createSignal("")
  const [error, setError] = createSignal("")

  let inputRef: HTMLInputElement | undefined

  const createGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.createGroup"]
  }))

  // Focus input on mount
  createEffect(() => {
    if (inputRef) {
      inputRef.focus()
    }
  })

  const handleCreate = async () => {
    const name = groupName().trim()

    if (!name) {
      setError(t("validation:_trn_field_required"))
      return
    }

    try {
      await createGroupMutation.mutateAsync(name)
      modals?.closeModal()
    } catch (e) {
      setError(t("general:_trn_error"))
    }
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleCreate()
    } else if (e.key === "Escape") {
      modals?.closeModal()
    }
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title}>
      <div class="flex flex-col gap-4 p-6 min-w-80">
        <div class="flex flex-col gap-2">
          <label class="text-sm text-lightSlate-400">
            <Trans key="instances:_trn_group_name" />
          </label>
          <Input
            ref={inputRef}
            value={groupName()}
            onInput={(e) => {
              setGroupName(e.currentTarget.value)
              setError("")
            }}
            onKeyDown={handleKeyDown}
            placeholder={t("instances:_trn_group_name_placeholder")}
            class="w-full"
          />
          {error() && <p class="text-red-400 text-sm m-0">{error()}</p>}
        </div>

        <div class="flex justify-end gap-2">
          <Button type="secondary" onClick={() => modals?.closeModal()}>
            <Trans key="general:_trn_cancel" />
          </Button>
          <Button
            type="primary"
            onClick={handleCreate}
            disabled={createGroupMutation.isPending || !groupName().trim()}
          >
            <Trans key="instances:_trn_create_group" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default CreateGroupModal
