import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { Show, createSignal, onCleanup } from "solid-js"
import { createJavaOverviewQuery } from "@/pages/Settings/settings.java.data"
import { javaChoices } from "@/pages/Settings/java/javaOverviewView"
import JavaSelect from "@/components/Java/JavaSelect"
import { CancelLink, DialogFooter } from "../DialogFooter"
import { TextLink } from "@/components/TextLink"

export interface JavaProfileCreationData {
  /** Runs once the dialog has closed, so the opener can put focus back. */
  onClosed?: () => void
}

const JavaProfileCreationModal = (props: ModalProps) => {
  const modalsContext = useModal()
  const [t] = useTransContext()
  const [profileName, setProfileName] = createSignal("")
  const [javaId, setJavaId] = createSignal<string | null>(null)

  onCleanup(() =>
    (props.data as JavaProfileCreationData | undefined)?.onClosed?.()
  )

  const createProfileMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createJavaProfile"]
  }))

  const allProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }))
  const overview = createJavaOverviewQuery()
  const javas = () => (overview.data ? javaChoices(overview.data) : [])

  const profileAlreadyExists = () =>
    (allProfiles.data || []).some((profile) => profile.name === profileName())

  const create = async () => {
    const id = javaId()
    if (!id) return
    try {
      await createProfileMutation.mutateAsync({
        profileName: profileName(),
        javaId: id
      })
    } catch {
      // The error toast says why; the dialog stays open to fix it.
      return
    }
    toast.success(t("java:_trn_profile_created"))
    modalsContext?.closeModal()
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-120"
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-4">
          <label class="flex flex-col gap-2">
            <span class="text-lightSlate-400 text-sm">
              <Trans key="general:_trn_profile_name" />
            </span>
            <Input
              autofocus
              disabled={createProfileMutation.isPending}
              placeholder={t("placeholders:_trn_type_profile_name")}
              // The same fill as the Java select under it.
              inputColor="bg-darkSlate-800"
              value={profileName()}
              onInput={(e) => setProfileName(e.currentTarget.value)}
              errorMessage={
                profileAlreadyExists()
                  ? t("errors:_trn_profile_name_exists")
                  : undefined
              }
            />
          </label>
          <div class="flex flex-col gap-2">
            <span class="text-lightSlate-400 text-sm">
              <Trans key="java:_trn_java" />
            </span>
            <JavaSelect
              javas={javas()}
              value={javaId()}
              label={t("java:_trn_java")}
              disabled={createProfileMutation.isPending}
              onChange={setJavaId}
            />
            <Show when={overview.isError && !overview.isFetching}>
              <div class="flex items-center gap-1.5 text-sm">
                <span class="text-red-300">
                  <Trans key="java:_trn_overview_failed" />
                </span>
                <span class="text-darkSlate-400">·</span>
                <TextLink onClick={() => overview.refetch()}>
                  <Trans key="java:_trn_try_again" />
                </TextLink>
              </div>
            </Show>
          </div>
        </div>
        <DialogFooter>
          <CancelLink
            disabled={createProfileMutation.isPending}
            onClick={() => modalsContext?.closeModal()}
          />
          <Button
            loading={createProfileMutation.isPending}
            disabled={profileAlreadyExists() || !javaId() || !profileName()}
            onClick={create}
          >
            <Trans key="general:_trn_create" />
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default JavaProfileCreationModal
