import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { For, Show } from "solid-js"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."

type Notice = NonNullable<
  Awaited<ReturnType<typeof window.takeRuntimeMoveNotice>>
>

const Files = (props: {
  intro: "left_changed" | "left_not_deleted"
  files: string[]
  count: number
}) => (
  <Show when={props.count}>
    <div>
      <p>
        <Trans key={`java:_trn_runtime_recovery.${props.intro}`} />
      </p>
      <ul class="bg-darkSlate-900 mt-2 max-h-32 overflow-y-auto break-all p-3 text-sm">
        <For each={props.files}>{(file) => <li>{file}</li>}</For>
      </ul>
      <Show when={props.count > props.files.length}>
        <p class="mt-1 text-sm">
          <Trans
            key="java:_trn_runtime_recovery.left_behind_more"
            options={{ number: props.count - props.files.length }}
          />
        </p>
      </Show>
    </div>
  </Show>
)

/** What a move of the data folder left in the old folder, and what to do with it. */
const RuntimeMoveNotice = (props: ModalProps) => {
  const modalsContext = useModal()
  const notice = () => props.data as Notice
  // Each start shows the notice again until the user acknowledges it here.
  const acknowledge = async () => {
    modalsContext?.closeModal()
    try {
      await window.dismissRuntimeMoveNotice()
    } catch (error) {
      console.error("Could not dismiss the data folder move notice", error)
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      width="w-180"
      onClose={() => void acknowledge()}
    >
      <div class="text-lightSlate-300 flex max-h-[70vh] flex-col gap-4 overflow-y-auto">
        <p>
          <Trans
            key={
              notice().keptOldFolder
                ? "java:_trn_runtime_recovery.kept_old_folder"
                : "java:_trn_runtime_recovery.left_behind"
            }
            options={{
              newPath: notice().newFolder,
              oldPath: notice().oldFolder,
              // Rendered as text; escaping would show "&#x2F;" for "/".
              interpolation: { escapeValue: false }
            }}
          />
        </p>
        <Files
          intro="left_changed"
          files={notice().changed}
          count={notice().changedCount}
        />
        <Files
          intro="left_not_deleted"
          files={notice().notDeleted}
          count={notice().notDeletedCount}
        />
        <div class="flex w-full justify-between">
          <Button
            type="secondary"
            onClick={() => window.openFolder(notice().oldFolder)}
          >
            <Trans key="java:_trn_runtime_recovery.open_old_folder" />
          </Button>
          <Button onClick={() => void acknowledge()}>
            <Trans key="general:_trn_ok" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default RuntimeMoveNotice
