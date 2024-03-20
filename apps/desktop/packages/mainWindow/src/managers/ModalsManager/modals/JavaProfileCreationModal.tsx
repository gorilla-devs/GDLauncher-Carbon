import { rspc } from "@/utils/rspcClient";
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button, Input, createNotification } from "@gd/ui";
import { Trans } from "@gd/i18n";
import { createSignal } from "solid-js";
import JavaPathAutoComplete from "@/components/JavaPathAutoComplete";

const JavaProfileCreationModal = (props: ModalProps) => {
  const modalsContext = useModal();
  const notification = createNotification();
  const [profileName, setProfileName] = createSignal("");
  const [javaId, setJavaId] = createSignal("");

  const createProfileMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createJavaProfile"]
  }));
  const createCustomJavaVersionMutation = rspc.createMutation(() => ({
    mutationKey: ["java.createCustomJavaVersion"]
  }));

  const allProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }));

  const profileAlreadyExists = () => {
    for (const profile of allProfiles.data || []) {
      if (profile.name === profileName()) return true;
    }

    return false;
  };

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-120"
      width="w-100"
    >
      <div class="flex flex-col justify-between h-full">
        <div class="flex flex-col gap-4">
          <h4>
            <Trans key="profile_name" />
          </h4>
          <Input
            disabled={createCustomJavaVersionMutation.isPending}
            placeholder="Type a profile name"
            value={profileName()}
            onInput={(e) => setProfileName(e.currentTarget.value)}
            error={
              profileAlreadyExists() ? "Profile name already exists" : undefined
            }
          />
          <h4>
            <Trans key="assigned_java_path" />
          </h4>
          <JavaPathAutoComplete
            inputColor="bg-darkSlate-600"
            disabled={createCustomJavaVersionMutation.isPending}
            updateValue={(value) => {
              if (value) setJavaId(value);
            }}
          />
        </div>
        <div class="flex justify-between">
          <Button
            type="secondary"
            disabled={createCustomJavaVersionMutation.isPending}
            onClick={() => {
              modalsContext?.closeModal();
            }}
          >
            <Trans key="instance_confirm_deletion.cancel" />
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
              });

              notification("Profile created successfully!", "success");

              modalsContext?.closeModal();
            }}
          >
            <Trans key="create" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default JavaProfileCreationModal;
