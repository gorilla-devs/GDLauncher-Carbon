import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createSignal } from "solid-js"
import JavaPathAutoComplete from "@/components/JavaPathAutoComplete"

const JavaProfileCreationModal = (props: ModalProps) => {
  const modalsContext = useModal()
  const [t] = useTransContext()
  const [profileName, setProfileName] = createSignal("")
  const [javaId, setJavaId] = createSignal("")

  const createProfileMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createJavaProfile"]
  }))
  const createCustomJavaVersionMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createCustomJavaVersion"]
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
            disabled={createCustomJavaVersionMutation.isPending}
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
            disabled={createCustomJavaVersionMutation.isPending}
            updateValue={(value) => {
              if (value) setJavaId(value)
            }}
          />
        </div>
        <div class="flex justify-between">
          <Button
            type="secondary"
            disabled={createCustomJavaVersionMutation.isPending}
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            <Trans key="instances:_trn_instance_confirm_deletion.cancel" />
          </Button>
          <Button
            disabled={
              profileAlreadyExists() ||
              !javaId() ||
              !profileName() ||
              createCustomJavaVersionMutation.isPending
            }
            onClick={async () => {
              await createProfileMutation.mutateAsync({
                profileName: profileName(),
                javaId: javaId()
              })

              toast.success("Profile created")

              modalsContext?.closeModal()
            }}
          >
            <Trans key="general:_trn_create" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default JavaProfileCreationModal
