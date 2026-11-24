import { useTransContext } from "@gd/i18n"
import { For, Show } from "solid-js"
import { useFormatSize } from "@/utils/java/useFormatSize"
import { ModalProps } from "../.."
import ModalLayout from "../../ModalLayout"
import ConfirmActions, {
  ConfirmCallbacks,
  createConfirm
} from "../../ConfirmActions"

export interface RemoveUnusedJavasConfirmData extends ConfirmCallbacks {
  javas: { id: string; name: string; sizeBytes: number | null }[]
  totalBytes: number
}

/** Lists the GDLauncher Javas nothing uses, and what removing them frees. */
const RemoveUnusedJavasConfirm = (props: ModalProps) => {
  const [t] = useTransContext()
  const size = useFormatSize()
  const data = () => props.data as RemoveUnusedJavasConfirmData
  const { busy, confirm, cancel } = createConfirm(data)

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={t("java:_trn_remove_unused_title")}
      width="w-120"
      height="h-auto"
      preventClose={busy()}
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-2">
          <ul class="m-0 flex max-h-60 list-none flex-col gap-1 overflow-y-auto p-0">
            <For each={data().javas}>
              {(java) => (
                <li class="flex items-center justify-between gap-4">
                  <span class="text-lightSlate-100 truncate">{java.name}</span>
                  <Show when={java.sizeBytes !== null}>
                    <span class="text-lightSlate-700 shrink-0 tabular-nums">
                      {size(java.sizeBytes!)}
                    </span>
                  </Show>
                </li>
              )}
            </For>
          </ul>
          <Show when={data().totalBytes > 0}>
            <div class="border-darkSlate-600 flex items-center justify-between gap-4 border-0 border-t border-solid pt-2">
              <span class="text-lightSlate-50">{t("java:_trn_total")}</span>
              <span class="text-lightSlate-50 shrink-0 tabular-nums">
                {size(data().totalBytes)}
              </span>
            </div>
          </Show>
        </div>
        <ConfirmActions busy={busy()} onConfirm={confirm} onCancel={cancel}>
          {t("java:_trn_remove_unused_confirm")}
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default RemoveUnusedJavasConfirm
