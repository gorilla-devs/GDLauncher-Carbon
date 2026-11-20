import { useTransContext } from "@gd/i18n"
import { ModalProps } from "../.."
import ModalLayout from "../../ModalLayout"
import ConfirmActions, {
  ConfirmCallbacks,
  createConfirm
} from "../../ConfirmActions"

export interface DeleteJavaProfileConfirmData extends ConfirmCallbacks {
  name: string
  /** How many instances name the profile; `null` when that's unknown. */
  users: number | null
}

/** Asks before deleting a custom profile that instances use. */
const DeleteJavaProfileConfirm = (props: ModalProps) => {
  const [t] = useTransContext()
  const data = () => props.data as DeleteJavaProfileConfirmData
  const { busy, confirm, cancel } = createConfirm(data)

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={t("java:_trn_delete_profile_title", { name: data().name })}
      width="w-120"
      height="h-auto"
      preventClose={busy()}
      compact
    >
      <div class="flex flex-col gap-6">
        <p class="text-lightSlate-300 m-0">
          {data().users === null
            ? t("java:_trn_profile_users_unknown")
            : t("java:_trn_profile_used_by", { count: data().users! })}
        </p>
        <ConfirmActions busy={busy()} onConfirm={confirm} onCancel={cancel}>
          {t("general:_trn_delete")}
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default DeleteJavaProfileConfirm
