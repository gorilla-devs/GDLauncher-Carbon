import { Button, Dropdown, Input, TextArea } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-110 flex flex-col justify-between">
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance.instance_name"
              options={{
                defaultValue: "Instance name",
              }}
            />
          </h5>
          <Input value="New instance" inputColor="bg-darkSlate-800" />
        </div>
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance.instance_description"
              options={{
                defaultValue: "Description",
              }}
            />
          </h5>
          <TextArea value="New instance" class="min-h-40" />
        </div>
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance.instance_loader"
              options={{
                defaultValue: "Loader",
              }}
            />
          </h5>
          <Dropdown
            options={[
              { label: t("instance.vanilla"), key: "vanilla" },
              { label: t("instance.forge"), key: "forge" },
              { label: t("instance.fabric"), key: "fabric" },
            ]}
            bgColorClass="bg-darkSlate-800"
            containerClass="w-full"
            class="w-full"
            value="vanilla"
          />
        </div>
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance.instance_version"
              options={{
                defaultValue: "Version",
              }}
            />
          </h5>
          <Dropdown
            options={[
              { label: "1.16.5", key: "1.16.5" },
              { label: "1.16.4", key: "1.16.4" },
              { label: "1.16.3", key: "1.16.3" },
              { label: "1.16.2", key: "1.16.2" },
            ]}
            bgColorClass="bg-darkSlate-800"
            containerClass="w-full"
            class="w-full"
            value="1.16.2"
          />
        </div>
        <div class="flex w-full justify-between">
          <Button
            variant="secondary"
            style={{ width: "100%", "max-width": "200px" }}
          >
            <Trans
              key="instance.instance_modal_instance_creation_cancel"
              options={{
                defaultValue: "Cancel",
              }}
            />
          </Button>
          <Button
            variant="primary"
            style={{ width: "100%", "max-width": "200px" }}
          >
            <Trans
              key="instance.instance_modal_instance_creation_create"
              options={{
                defaultValue: "Create",
              }}
            />
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default InstanceCreation;
