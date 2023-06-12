import { Button, Checkbox, Dropdown, Input, createNotification } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { port, rspc } from "@/utils/rspcClient";
import {
  FEModdedManifestLoaderVersion,
  FEModdedManifestVersion,
  ManifestVersion,
  ModLoaderType,
} from "@gd/core_module/bindings";
import { blobToBase64 } from "@/utils/helpers";
import { mcVersions } from "@/utils/mcVersion";

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();
  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    FEModdedManifestVersion[] | ManifestVersion[]
  >([]);
  const [title, setTitle] = createSignal("");
  const [error, setError] = createSignal("");
  const [bgPreview, setBgPreview] = createSignal<string | null>(null);
  const [loader, setLoader] = createSignal<ModLoaderType | undefined>(
    undefined
  );
  const [loaderVersions, setLoaderVersions] = createSignal<
    FEModdedManifestLoaderVersion[]
  >([]);
  const [chosenLoaderVersion, setChosenLoaderVersion] = createSignal("");
  const [mcVersionsHashMap, setMcVersionsHashMap] = createSignal<{
    [id: string]: string;
  }>({});
  const [mcVersion, setMcVersion] = createSignal("");
  const [snapshotVersionFilter, setSnapshotVersionFilter] = createSignal(true);
  const [releaseVersionFilter, setReleaseVersionFilter] = createSignal(true);
  const [oldBetaVersionFilter, setOldBetaVersionFilter] = createSignal(true);
  const [oldAlphaVersionFilter, setOldAlphaVersionFilter] = createSignal(true);

  const addNotification = createNotification();
  const modalsContext = useModal();

  const forgeVersionsQuery = rspc.createQuery(() => ["mc.getForgeVersions"], {
    enabled: false,
  });

  createEffect(() => {
    mcVersions().forEach((v) => {
      setMcVersionsHashMap((prev) => ({ ...prev, [v.id]: v.type }));
    });
  });

  createEffect(() => {
    if (forgeVersionsQuery.data && loader() === "Forge") {
      const filteredVersions = forgeVersionsQuery.data.gameVersions.filter(
        (v) => mcVersionsHashMap()[v.id]
      );

      setMappedMcVersions(filteredVersions);

      const mcVers = forgeVersionsQuery?.data?.gameVersions[0];
      const versions = forgeVersionsQuery?.data?.gameVersions.find(
        (v) => v.id === mcVers.id
      )?.loaders;

      if (versions) setLoaderVersions(versions);
    } else if (!loader()) {
      setMappedMcVersions(mcVersions());
      setLoaderVersions([]);
    }
  });

  createEffect(() => {
    const filteredVersions = mcVersions().filter((v) => {
      if (
        releaseVersionFilter() &&
        snapshotVersionFilter() &&
        oldAlphaVersionFilter() &&
        oldBetaVersionFilter()
      ) {
        return true;
      } else if (releaseVersionFilter()) {
        return v.type === "release";
      } else if (snapshotVersionFilter()) {
        return v.type === "snapshot";
      } else if (oldAlphaVersionFilter()) {
        return v.type === "old_alpha";
      } else if (oldBetaVersionFilter()) {
        return v.type === "old_beta";
      } else if (
        !releaseVersionFilter() &&
        !snapshotVersionFilter() &&
        !oldAlphaVersionFilter() &&
        !oldBetaVersionFilter()
      ) {
        return true;
      }
    });

    setMappedMcVersions(filteredVersions);
  });

  const modloaders = [
    { label: t("instance.vanilla"), key: undefined },
    { label: t("instance.forge"), key: "Forge" },
    // { label: t("instance.fabric"), key: "Fabric" },
  ];

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        modalsContext?.closeModal();
        addNotification("Instance successfully created.");
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
        modalsContext?.closeModal();
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        setError("");
        setTitle("");
        setError("");
        setBgPreview(null);
        setMcVersion("");
        setChosenLoaderVersion("");
      },
    }
  );

  const handleCreate = () => {
    if (!title()) {
      setError("Fields must be filled in!");
    } else {
      setError("");

      const mcVers = forgeVersionsQuery?.data?.gameVersions[0];
      const versions =
        forgeVersionsQuery?.data?.gameVersions.find((v) => v.id === mcVers?.id)
          ?.loaders || [];

      createInstanceMutation.mutate({
        group: defaultGroup.data || 1,
        use_loaded_icon: true,
        notes: "",
        name: title(),
        version: {
          Version: {
            Standard: {
              release: mcVersion() || (mappedMcVersions()?.[0]?.id as string),
              modloaders: loader()
                ? [
                    {
                      type_: loader() as ModLoaderType,
                      version: chosenLoaderVersion() || versions[0].id,
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
                      if (modloader.key === "Forge") {
                        forgeVersionsQuery.refetch();
                      } else if (modloader.key === "Fabric") {
                        // fabric
                      }

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
                    disabled={forgeVersionsQuery.isLoading}
                    options={mappedMcVersions().map((v) => ({
                      label: v.id,
                      key: v.id,
                    }))}
                    bgColorClass="bg-darkSlate-800"
                    containerClass="w-full"
                    class="w-full"
                    placement="bottom"
                    onChange={(l) => {
                      setMcVersion(l.key as string);

                      if (!loader) {
                        setLoaderVersions([]);
                      } else if (loader() === "Forge") {
                        const versions =
                          forgeVersionsQuery?.data?.gameVersions.find(
                            (v) => v.id === l.key
                          )?.loaders;

                        if (versions) setLoaderVersions(versions);
                      }
                    }}
                  />
                </Show>
                <Show when={!loader()}>
                  <div class="flex gap-4 mt-2">
                    <div class="flex gap-2 items-center">
                      <Checkbox
                        checked={snapshotVersionFilter()}
                        onChange={(e) => setSnapshotVersionFilter(e)}
                      />
                      <h6 class="m-0 flex items-center">
                        <Trans
                          key="instance.instance_version_snapshot"
                          options={{
                            defaultValue: "Snapshot",
                          }}
                        />
                      </h6>
                    </div>
                    <div class="flex gap-2">
                      <Checkbox
                        checked={releaseVersionFilter()}
                        onChange={(e) => setReleaseVersionFilter(e)}
                      />
                      <h6 class="m-0 flex items-center">
                        <Trans
                          key="instance.instance_version_release"
                          options={{
                            defaultValue: "Release",
                          }}
                        />
                      </h6>
                    </div>
                    <div class="flex gap-2">
                      <Checkbox
                        checked={oldAlphaVersionFilter()}
                        onChange={(e) => setOldAlphaVersionFilter(e)}
                      />
                      <h6 class="m-0 flex items-center">
                        <Trans
                          key="instance.instance_version_old_alphas"
                          options={{
                            defaultValue: "Old alpha",
                          }}
                        />
                      </h6>
                    </div>
                    <div class="flex gap-2">
                      <Checkbox
                        checked={oldBetaVersionFilter()}
                        onChange={(e) => setOldBetaVersionFilter(e)}
                      />
                      <h6 class="m-0 flex items-center">
                        <Trans
                          key="instance.instance_version_old_beta"
                          options={{
                            defaultValue: "Old beta",
                          }}
                        />
                      </h6>
                    </div>
                  </div>
                </Show>
              </div>
            </div>
            <Show when={loaderVersions().length > 0}>
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
                  disabled={!loaderVersions()}
                  options={loaderVersions()?.map((v) => ({
                    label: v.id,
                    key: v.id,
                  }))}
                  bgColorClass="bg-darkSlate-800"
                  containerClass="w-full"
                  class="w-full"
                  value={loaderVersions()[0].id}
                  placement="bottom"
                  onChange={(l) => {
                    if (loader() === "Forge") {
                      const key = l.key as string;
                      const versions =
                        forgeVersionsQuery?.data?.gameVersions.find(
                          (v) => v.id === key
                        )?.loaders;
                      if (versions) setLoaderVersions(versions);
                    }
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
