import { Button, Dropdown, Input, TextArea } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";
import { createEffect, createSignal } from "solid-js";
import { mcVersions } from "@/utils/mcVersion";

type MappedMcVersion = {
  label: string;
  key: string;
};

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();

  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    MappedMcVersion[]
  >([]);

  createEffect(() => {
    const versions = mcVersions().map((version) => ({
      label: `${version.id} - ${version.type}`,
      key: version.id,
    }));
    setMappedMcVersions(versions);
  });

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 flex flex-col justify-between w-110">
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
          <TextArea value="New instance" class="min-h-40 resize-none" />
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
            options={mappedMcVersions()}
            bgColorClass="bg-darkSlate-800"
            containerClass="w-full"
            class="w-full"
            value={mappedMcVersions()[0].key}
            placement="bottom"
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
