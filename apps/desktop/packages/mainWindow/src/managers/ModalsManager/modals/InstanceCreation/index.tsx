import { Button, Dropdown, Input, TextArea } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans } from "@gd/i18n";

const InstanceCreation = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-110 flex flex-col justify-between">
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance_name"
              options={{
                defaultValue: "Instance name",
              }}
            />
          </h5>
          <Input value="New instance" inputColor="bg-shade-8" />
        </div>
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance_description"
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
              key="instance_loader"
              options={{
                defaultValue: "Loader",
              }}
            />
          </h5>
          <Dropdown
            options={[
              { label: "vanilla", key: "vanilla" },
              { label: "forge", key: "forge" },
              { label: "fabric", key: "fabric" },
            ]}
            bg="bg-shade-8"
            containerClass="w-full"
            class="w-full"
            value="vanilla"
          />
        </div>
        <div>
          <h5 class="mt-0 mb-2">
            <Trans
              key="instance_version"
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
            bg="bg-shade-8"
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
              key="create_instance_modal_cancel"
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
              key="create_instance_modal_create"
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
