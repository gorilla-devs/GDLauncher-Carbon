import { Button, Dropdown, Input, TextArea } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";
import { Show, createEffect, createSignal } from "solid-js";
import { mcVersions } from "@/utils/mcVersion";
import { rspc } from "@/utils/rspcClient";
import { ModLoaderType } from "@gd/core_module/bindings";

type MappedMcVersion = {
  label: string;
  key: string;
};

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();
  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    MappedMcVersion[]
  >([]);
  const [title, setTitle] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [loader, setLoader] = createSignal<ModLoaderType | undefined>(
    undefined
  );
  const [mcVersion, setMcVersion] = createSignal("");

  createEffect(() => {
    const versions = mcVersions().map((version) => ({
      label: `${version.id} - ${version.type}`,
      key: version.id,
    }));
    setMappedMcVersions(versions);
  });

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onSuccess() {
        console.log("SUCCESS INSTANCE CREWATION");
      },
      onError() {
        console.log("ERROR INSTANCE CREWATION");
      },
    }
  );

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
          <Input
            placeholder="New instance"
            inputColor="bg-darkSlate-800"
            onInput={(e) => {
              setTitle(e.currentTarget.value);
            }}
          />
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
          <TextArea
            placeholder="New instance"
            class="min-h-40 resize-none"
            onInput={(e) => {
              setDescription(e.currentTarget.value);
            }}
          />
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
              { label: t("instance.vanilla"), key: "Vanilla" },
              { label: t("instance.forge"), key: "Forge" },
              { label: t("instance.fabric"), key: "Fabric" },
            ]}
            bgColorClass="bg-darkSlate-800"
            containerClass="w-full"
            class="w-full"
            value="vanilla"
            onChange={(loader) => {
              if (loader.key !== "Vanilla") {
                setLoader(loader.key as ModLoaderType);
              }
            }}
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
          <Show when={mappedMcVersions().length > 0}>
            <Dropdown
              options={mappedMcVersions()}
              bgColorClass="bg-darkSlate-800"
              containerClass="w-full"
              class="w-full"
              value={mappedMcVersions()[0].key}
              placement="bottom"
              onChange={(loader) => {
                setMcVersion(loader.key);
              }}
            />
          </Show>
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
            onClick={() => {
              createInstanceMutation.mutate({
                group: defaultGroup.data || 1,
                icon: "",
                name: title(),
                version: {
                  Version: {
                    Standard: {
                      release: mcVersion(),
                      modloaders: loader()
                        ? [{ type_: loader() as ModLoaderType, version: "" }]
                        : [],
                    },
                  },
                },
              });
            }}
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
