import { Button, Dropdown, Input, TextArea, createNotification } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";
import { Show, createEffect, createSignal } from "solid-js";
import { mcVersions } from "@/utils/mcVersion";
import { rspc } from "@/utils/rspcClient";
import { ModLoaderType } from "@gd/core_module/bindings";
import { blobToBase64 } from "@/utils/helpers";

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
  const [error, setError] = createSignal("");
  const [imageFilePath, setImageFilePath] = createSignal("");
  const [bgPreview, setBgPreview] = createSignal("");
  const [loader, setLoader] = createSignal<ModLoaderType | undefined>(
    undefined
  );
  const [mcVersion, setMcVersion] = createSignal("");

  const addNotification = createNotification();
  const modalsContext = useModal();

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
        addNotification("Instance saccessfully created.");
        modalsContext?.closeModal();
      },
      onError() {
        addNotification("Error while creating the instance.");
        modalsContext?.closeModal();
      },
    }
  );

  const handleCreate = () => {
    if (
      !title() ||
      !description() ||
      !mcVersion() ||
      !loader() ||
      !imageFilePath()
    ) {
      setError("Fields must be filled in!");
    } else {
      setError("");
      createInstanceMutation.mutate({
        group: defaultGroup.data || 1,
        icon: imageFilePath(),
        name: title(),
        version: {
          Version: {
            Standard: {
              release: mcVersion(),
              modloaders: loader()
                ? [
                    {
                      type_: loader() as ModLoaderType,
                      version: "1.2.3",
                    },
                  ]
                : [],
            },
          },
        },
      });
    }
  };

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
    >
      <div class="flex flex-col justify-between overflow-y-scroll max-h-128 w-120">
        <div class="flex flex-col justify-between gap-4 p-5">
          <div
            class="flex justify-center items-center w-20 bg-darkSlate-900 rounded-xl cursor-pointer h-20"
            style={{
              "background-image": `url("/Users/ladvace/Desktop/memoji.png")`,
            }}
            onClick={() => {
              window
                .openFileDialog([
                  { name: "Image", extensions: ["png", "jpg", "jpeg"] },
                ])
                .then((files) => {
                  // const blob = new Blob([files], {
                  //   type: "application/octet-stream",
                  // });

                  // blobToBase64(blob).then(b64);

                  setImageFilePath(files.filePaths[0]);
                });
            }}
          >
            <h3 class="text-center">
              <Trans
                key="instance.upload_image"
                options={{
                  defaultValue: "Upload image",
                }}
              />
            </h3>
          </div>

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
              required
              placeholder="New instance"
              inputColor="bg-darkSlate-800"
              onInput={(e) => {
                setTitle(e.currentTarget.value);
              }}
              error={
                error() &&
                !title() &&
                (t("error.missing_field_title") as string)
              }
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
              required
              placeholder="New instance"
              class="min-h-40 resize-none"
              onInput={(e) => {
                setDescription(e.currentTarget.value);
              }}
              error={
                error() &&
                !description() &&
                (t("error.missing_field_description") as string)
              }
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
              error={
                error() &&
                !loader() &&
                (t("error.missing_field_loader") as string)
              }
            />
          </div>
          <div>
            <h5 class="mt-0 mb-2">
              <Trans
                key="instance.instance_loader_version"
                options={{
                  defaultValue: "Loader version",
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
                error={
                  error() &&
                  !loader() &&
                  (t("error.missing_field_loader_version") as string)
                }
              />
            </Show>
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
                error={
                  error() &&
                  !mcVersion() &&
                  (t("error.missing_field_mc_version") as string)
                }
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
                handleCreate();
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
      </div>
    </ModalLayout>
  );
};

export default InstanceCreation;
