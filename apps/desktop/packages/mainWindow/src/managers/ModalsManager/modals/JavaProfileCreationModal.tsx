import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createSignal } from "solid-js"
import JavaPathAutoComplete, {
  type JavaPathSelection
} from "@/components/JavaPathAutoComplete"

const JavaProfileCreationModal = (props: ModalProps) => {
  const modalsContext = useModal()
  const [t] = useTransContext()
  const [profileName, setProfileName] = createSignal("")
  const [selection, setSelection] = createSignal<JavaPathSelection>({
    id: null,
    path: "",
    pending: false
  })
  const [submitting, setSubmitting] = createSignal(false)

  const createProfileMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createJavaProfile"]
  }))

  const allProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }))

  const profileAlreadyExists = () => {
    for (const profile of allProfiles.data || []) {
      if (profile.name === profileName()) return true
    }

    return false
  }

  const canCreate = () =>
    !submitting() &&
    !createProfileMutation.isPending &&
    !profileAlreadyExists() &&
    !!profileName() &&
    !!selection().id &&
    !!selection().path &&
    !selection().pending
  const createProfile = async () => {
    if (!canCreate()) return
    const captured = { ...selection() }
    const name = profileName()
    setSubmitting(true)
    try {
      if (
        selection().id !== captured.id ||
        selection().path !== captured.path ||
        selection().pending
      )
        return
      await createProfileMutation.mutateAsync({
        profileName: name,
        javaId: captured.id!
      })
      toast.success("Profile created")
      modalsContext?.closeModal()
    } catch {
      // MutationCache owns RPC error feedback; keep this event handler settled.
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-120"
      width="w-100"
    >
      <div class="flex h-full flex-col justify-between">
        <div class="flex flex-col gap-4">
          <h4>
            <Trans key="general:_trn_profile_name" />
          </h4>
          <Input
            disabled={submitting() || createProfileMutation.isPending}
            placeholder={t("placeholders:_trn_type_profile_name")}
            value={profileName()}
            onInput={(e) => setProfileName(e.currentTarget.value)}
            errorMessage={
              profileAlreadyExists()
                ? t("errors:_trn_profile_name_exists")
                : undefined
            }
          />
          <h4>
            <Trans key="general:_trn_assigned_java_path" />
          </h4>
          <JavaPathAutoComplete
            inputColor="bg-darkSlate-600"
            disabled={submitting() || createProfileMutation.isPending}
            onSelection={setSelection}
          />
        </div>
        <div class="flex justify-between">
          <Button
            type="secondary"
            disabled={submitting() || createProfileMutation.isPending}
            onClick={() => {
              if (submitting() || createProfileMutation.isPending) return
              modalsContext?.closeModal()
            }}
          >
            <Trans key="instances:_trn_instance_confirm_deletion.cancel" />
          </Button>
          <Button disabled={!canCreate()} onClick={createProfile}>
            <Trans key="general:_trn_create" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default JavaProfileCreationModal
