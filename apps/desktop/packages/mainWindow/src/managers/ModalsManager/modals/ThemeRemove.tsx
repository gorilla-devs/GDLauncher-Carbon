import { Trans, useTransContext } from "@gd/i18n"
import { createSignal, Show } from "solid-js"
import ModalLayout from "../ModalLayout"
import ConfirmActions from "../ConfirmActions"
import { ModalProps, useModal } from ".."
import { extractErrorCode, rspc } from "@/utils/rspcClient"
import { themeRefusalKey, type ThemeErrorKey } from "@/themes/errors"

// Removing asks first: a folder placed by hand may be its author's only copy.
const ThemeRemove = (props: ModalProps) => {
  const [t] = useTransContext()
  const modals = useModal()
  const id = (): string => props.data?.id ?? ""
  const name = (): string => props.data?.name ?? ""
  const [error, setError] = createSignal<ThemeErrorKey | null>(null)

  const removeMutation = rspc.createMutation(() => ({
    mutationKey: ["themes.remove"]
  }))

  const remove = async () => {
    if (removeMutation.isPending) return
    setError(null)
    try {
      await removeMutation.mutateAsync({ id: id() })
    } catch (e) {
      // A disk failure has no code: the global error toast reports it.
      setError(themeRefusalKey(extractErrorCode(e)))
      return
    }
    modals?.closeModal("themeRemove")
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title} width="w-120">
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-2">
          <p class="text-lightSlate-300 m-0 break-words">
            {t("settings:_trn_theme_remove_text", {
              name: name(),
              interpolation: { escapeValue: false }
            })}
          </p>
          <Show when={error()}>
            {(key) => <p class="m-0 text-red-400">{t(key())}</p>}
          </Show>
        </div>
        <ConfirmActions
          busy={removeMutation.isPending}
          confirmTestId="theme-remove-confirm"
          onCancel={() => modals?.closeModal("themeRemove")}
          onConfirm={() => void remove()}
        >
          <Trans key="settings:_trn_theme_remove" />
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default ThemeRemove
