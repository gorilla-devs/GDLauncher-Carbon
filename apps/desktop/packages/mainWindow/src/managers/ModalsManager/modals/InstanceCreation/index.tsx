import { Button, Dropdown, Input, createNotification } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { forgeVersions, mcVersions } from "@/utils/mcVersion";
import { port, rspc } from "@/utils/rspcClient";
import { ModLoaderType } from "@gd/core_module/bindings";
import { blobToBase64 } from "@/utils/helpers";

type MappedVersion = {
  label: string;
  key: string;
};

interface MappedMcVersion extends MappedVersion {
  type: string;
}

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();
  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    MappedMcVersion[]
  >([]);
  const [title, setTitle] = createSignal("");
  const [error, setError] = createSignal("");
  const [bgPreview, setBgPreview] = createSignal<string | null>(null);
  const [loader, setLoader] = createSignal<ModLoaderType | undefined>(
    undefined
  );
  const [loaderVersions, setLoaderVersions] = createSignal<
    MappedVersion[] | undefined
  >(undefined);
  const [mcVersion, setMcVersion] = createSignal("");
  const [loaderVersion, setLoaderVersion] = createSignal("");
  const [releasVersion, setReleaseVersion] = createSignal("release");

  const addNotification = createNotification();
  const modalsContext = useModal();

  createEffect(() => {
    const mcVersionLoaderVersions = forgeVersions()?.gameVersions.find(
      (version) =>
        version.id === (mcVersion() || (mappedMcVersions()?.[0]?.key as string))
    )?.loaders;

    const mappedVersion = mcVersionLoaderVersions?.map((version) => ({
      label: version.id,
      key: version.id,
    }));
    setLoaderVersions(mappedVersion);
  });

  createEffect(() => {
    const versions = mcVersions().map((version) => ({
      label: version.id,
      key: version.id,
      type: version.type,
    }));

    const filteredVersions = versions.filter((v) => {
      const isOthers = releasVersion() === "others";
      return isOthers
        ? v.type !== "release" && v.type !== "snapshot"
        : v.type === releasVersion();
    });

    setMappedMcVersions(filteredVersions);
  });

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        addNotification("Instance successfully created.");
        modalsContext?.closeModal();
      },

      onError() {
        addNotification("Error while creating the instance.", "error");
        modalsContext?.closeModal();
      },
    }
  );

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onSuccess(instanceId) {
        prepareInstanceMutation.mutate(instanceId);
      },
      onError() {
        addNotification("Error while creating the instance.", "error");
        modalsContext?.closeModal();
      },
      onSettled() {
        setError("");
        setTitle("");
        setError("");
        setBgPreview(null);
        setMcVersion("");
        setLoaderVersion("");
      },
    }
  );

  const handleCreate = () => {
    if (!title()) {
      setError("Fields must be filled in!");
    } else {
      setError("");

      createInstanceMutation.mutate({
        group: defaultGroup.data || 1,
        use_loaded_icon: true,
        notes: "",
        name: title(),
        version: {
          Version: {
            Standard: {
              release: mcVersion() || (mappedMcVersions()?.[0]?.key as string),
              modloaders: loader()
                ? [
                    {
                      type_: loader() as ModLoaderType,
                      version:
                        loaderVersion() ||
                        (loaderVersions()?.[0]?.key as string),
                    },
                  ]
                : [],
            },
          },
        },
      });
    }
  };

  const modloaders = [
    { label: t("instance.vanilla"), key: undefined },
    { label: t("instance.forge"), key: "Forge" },
    // { label: t("instance.fabric"), key: "Fabric" },
  ];

  const releaseVersions = [
    { label: t("instance.instance_version_release"), key: "release" },
    { label: t("instance.instance_version_snapshot"), key: "snapshot" },
    { label: t("instance.instance_version_others"), key: "others" },
  ];

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
    >
      <div class="flex flex-col justify-between scrollbar-hide overflow-y-scroll w-120 h-136">
        <div class="flex flex-col justify-between gap-4 p-5 h-full">
          <span class="flex flex-col justify-between gap-4">
            <div
              class="relative flex justify-center items-center bg-darkSlate-900 cursor-pointer bg-center bg-cover rounded-xl w-20 h-20"
              style={{
                "background-image": `url("${bgPreview()}")`,
              }}
              onClick={() => {
                if (bgPreview()) return;
                window
                  .openFileDialog([
                    { name: "Image", extensions: ["png", "jpg", "jpeg"] },
                  ])
                  .then((files) => {
                    if (!files.filePaths[0]) return;
                    fetch(
                      `http://localhost:${port}/instance/loadIcon?path=${files.filePaths[0]}`
                    ).then(async (img) => {
                      const blob = await img.blob();
                      const b64 = (await blobToBase64(blob)) as string;

                      setBgPreview(
                        `data:image/png;base64, ${b64.substring(
                          b64.indexOf(",") + 1
                        )}`
                      );
                    });
                  });
              }}
            >
              <Switch>
                <Match when={!bgPreview()}>
                  <h3 class="text-center">
                    <Trans
                      key="instance.upload_image"
                      options={{
                        defaultValue: "Upload image",
                      }}
                    />
                  </h3>
                </Match>
                <Match when={bgPreview()}>
                  <div class="absolute top-0 right-0 pl-2 pb-2 bg-darkSlate-700 rounded-bl-2xl">
                    <div
                      class="text-white transition-all duration-100 ease-in-out text-lg i-ri:close-circle-fill hover:color-red-500"
                      onClick={(e) => {
                        e.preventDefault();
                        setBgPreview(null);
                      }}
                    />
                  </div>
                </Match>
              </Switch>
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
                value={title()}
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
                  key="instance.instance_loader"
                  options={{
                    defaultValue: "Loader",
                  }}
                />
              </h5>
              <div class="flex gap-2">
                <For each={modloaders}>
                  {(modloader) => (
                    <div
                      class="px-3 py-2 bg-darkSlate-800 rounded-lg cursor-pointer border-box"
                      classList={{
                        "border-2 border-solid border-transparent":
                          loader() !== modloader.key,
                        "border-2 border-solid border-primary-500":
                          loader() === modloader.key,
                      }}
                      onClick={() => {
                        setLoader(
                          modloader.key === "Vanilla"
                            ? undefined
                            : (modloader.key as ModLoaderType)
                        );
                      }}
                    >
                      {modloader.label}
                    </div>
                  )}
                </For>
              </div>
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
              <div class="flex gap-4 mt-2">
                <For each={releaseVersions}>
                  {(release) => (
                    <div
                      class="px-3 py-2 bg-darkSlate-800 rounded-lg cursor-pointer border-box"
                      classList={{
                        "border-2 border-solid border-transparent":
                          releasVersion() !== release.key,
                        "border-2 border-solid border-primary-500":
                          releasVersion() === release.key,
                      }}
                      onClick={() => {
                        setReleaseVersion(release.key);
                      }}
                    >
                      {release.label}
                    </div>
                  )}
                </For>
              </div>
            </div>
            <div>
              <h5 class="mt-0 mb-2">
                <Trans
                  key="instance.instance_mc_version"
                  options={{
                    defaultValue: "Minecraft Version",
                  }}
                />
              </h5>
              <div>
                <Show when={mappedMcVersions().length > 0}>
                  <Dropdown
                    options={mappedMcVersions()}
                    bgColorClass="bg-darkSlate-800"
                    containerClass="w-full"
                    class="w-full"
                    placement="bottom"
                    onChange={(loader) => {
                      setMcVersion(loader.key as string);
                    }}
                  />
                </Show>
              </div>
            </div>
            <Show when={loaderVersions() && loader() === "Forge"}>
              <div>
                <h5 class="mt-0 mb-2">
                  <Trans
                    key="instance.instance_loader_version"
                    options={{
                      defaultValue: "Loader version",
                    }}
                  />
                </h5>
                <Dropdown
                  options={loaderVersions() as MappedVersion[]}
                  bgColorClass="bg-darkSlate-800"
                  containerClass="w-full"
                  class="w-full"
                  disabled={!loaderVersions()}
                  value={loaderVersions()?.[0]?.key}
                  placement="bottom"
                  onChange={(loader) => {
                    setLoaderVersion(loader.key as string);
                  }}
                />
              </div>
            </Show>
          </span>
          <div class="flex w-full justify-between">
            <Button
              type="secondary"
              style={{ width: "100%", "max-width": "200px" }}
              onClick={() => modalsContext?.closeModal()}
            >
              <Trans
                key="instance.instance_modal_instance_creation_cancel"
                options={{
                  defaultValue: "Cancel",
                }}
              />
            </Button>
            <Button
              type="primary"
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
