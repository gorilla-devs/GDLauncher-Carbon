import { Button, Checkbox, Dropdown, Input, createNotification } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import { Trans, useTransContext } from "@gd/i18n";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { port, rspc } from "@/utils/rspcClient";
import {
  FEModdedManifestLoaderVersion,
  ManifestVersion,
  McType,
  CFFEModLoaderType,
} from "@gd/core_module/bindings";
import { blobToBase64 } from "@/utils/helpers";
import { mcVersions } from "@/utils/mcVersion";
import { useGDNavigate } from "@/managers/NavigationManager";
import { trackEvent } from "@/utils/analytics";
import { ReactiveMap } from "@solid-primitives/map";

type MappedMcVersions = ManifestVersion & { hasModloader?: boolean };

type Instancetype = {
  id: string;
  modloader: CFFEModLoaderType | undefined;
  title: string | undefined;
  mcVersion: string | undefined;
  modloaderVersion: string | undefined;
  img: string | undefined;
};

// eslint-disable-next-line no-unused-vars
enum Modloaders {
  _Quilt = "quilt",
  _Forge = "forge",
  _Fabric = "fabric",
}

const Custom = (props: Pick<ModalProps, "data">) => {
  const [t] = useTransContext();
  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    MappedMcVersions[]
  >([]);

  const instanceData = () => props.data as Instancetype | undefined;

  const [error, setError] = createSignal("");
  const [bgPreview, setBgPreview] = createSignal<string | null>(
    instanceData()?.img || null
  );
  const [loader, setLoader] = createSignal<CFFEModLoaderType | undefined>(
    instanceData()?.modloader || undefined
  );
  const [loaderVersions, setLoaderVersions] = createSignal<
    FEModdedManifestLoaderVersion[]
  >([]);
  const [chosenLoaderVersion, setChosenLoaderVersion] = createSignal(
    instanceData()?.modloaderVersion || ""
  );
  const [mcVersion, setMcVersion] = createSignal(
    instanceData()?.mcVersion || ""
  );
  const [title, setTitle] = createSignal(instanceData()?.title || "");
  const [releaseVersionFilter, setReleaseVersionFilter] = createSignal(true);
  const [snapshotVersionFilter, setSnapshotVersionFilter] = createSignal(false);
  const [oldBetaVersionFilter, setOldBetaVersionFilter] = createSignal(false);
  const [oldAlphaVersionFilter, setOldAlphaVersionFilter] = createSignal(false);

  const forgeHashmap = new ReactiveMap();
  const fabricHashmap = new ReactiveMap();
  const quiltHashmap = new ReactiveMap();

  const addNotification = createNotification();
  const modalsContext = useModal();
  const navigate = useGDNavigate();

  const forgeVersionsQuery = rspc.createQuery(() => ["mc.getForgeVersions"], {
    enabled: false,
    onSuccess(data) {
      data.gameVersions.forEach((version) => {
        forgeHashmap.set(version.id, version.loaders);
      });
    },
  });

  const fabricVersionsQuery = rspc.createQuery(() => ["mc.getFabricVersions"], {
    enabled: false,
    onSuccess(data) {
      data.gameVersions.forEach((version) => {
        fabricHashmap.set(version.id, version.loaders);
      });
    },
  });

  const quiltVersionsQuery = rspc.createQuery(() => ["mc.getQuiltVersions"], {
    enabled: false,
    onSuccess(data) {
      data.gameVersions.forEach((version) => {
        quiltHashmap.set(version.id, version.loaders);
      });
    },
  });

  const DUMMY_META_VERSION = "${gdlauncher.gameVersion}";

  const isFabric = () => loader() === Modloaders._Fabric;
  const isForge = () => loader() === Modloaders._Forge;
  const isQuilt = () => loader() === Modloaders._Quilt;

  createEffect(() => {
    if (forgeVersionsQuery.data && isForge()) {
      const versions = forgeVersionsQuery?.data?.gameVersions.find(
        (v) => v.id === (mcVersion() || (mappedMcVersions()?.[0]?.id as string))
      )?.loaders;

      setLoaderVersions(versions || []);
    } else if (!loader()) {
      setLoaderVersions([]);
    }
  });

  createEffect(() => {
    if (fabricVersionsQuery.data && isFabric()) {
      const supported =
        fabricVersionsQuery?.data?.gameVersions.find(
          (v) =>
            v.id === (mcVersion() || (mappedMcVersions()?.[0]?.id as string))
        ) ?? false;

      const versions =
        supported !== false
          ? fabricVersionsQuery?.data?.gameVersions.find(
              (v) => v.id === DUMMY_META_VERSION
            )?.loaders
          : [];

      setLoaderVersions(versions || []);
    } else if (!loader()) {
      setLoaderVersions([]);
    }
  });

  createEffect(() => {
    if (quiltVersionsQuery.data && isQuilt()) {
      const supported =
        quiltVersionsQuery?.data?.gameVersions.find(
          (v) =>
            v.id === (mcVersion() || (mappedMcVersions()?.[0]?.id as string))
        ) ?? false;

      const versions =
        supported !== false
          ? quiltVersionsQuery?.data?.gameVersions.find(
              (v) => v.id === DUMMY_META_VERSION
            )?.loaders
          : [];

      setLoaderVersions(versions || []);
    } else if (!loader()) {
      setLoaderVersions([]);
    }
  });

  createEffect(() => {
    const filteredData = mcVersions().filter(
      (item) =>
        (item.type === "release" && releaseVersionFilter()) ||
        (item.type === "snapshot" && snapshotVersionFilter()) ||
        (item.type === "old_beta" && oldBetaVersionFilter()) ||
        (item.type === "old_alpha" && oldAlphaVersionFilter())
    );

    const forgeMappedVersions = filteredData.map((item) => {
      return { ...item, hasModloader: forgeHashmap.has(item.id) };
    });
    const fabricMappedVersions = filteredData.map((item) => {
      return { ...item, hasModloader: fabricHashmap.has(item.id) };
    });
    const quiltMappedVersions = filteredData.map((item) => {
      return { ...item, hasModloader: quiltHashmap.has(item.id) };
    });

    if (isForge()) setMappedMcVersions(forgeMappedVersions);
    else if (isFabric()) setMappedMcVersions(fabricMappedVersions);
    else if (isQuilt()) setMappedMcVersions(quiltMappedVersions);
    else setMappedMcVersions(filteredData);
  });

  const autoGeneratedName = () =>
    `${loader() || "Vanilla"} ${
      mcVersion() || (mappedMcVersions()?.[0]?.id as string)
    }`;

  createEffect(() => {
    setTitle(autoGeneratedName());
  });

  const modloaders: {
    label: string;
    key: CFFEModLoaderType | undefined;
  }[] = [
    { label: t("instance.vanilla"), key: undefined },
    { label: t("instance.forge"), key: "forge" },
    { label: t("instance.fabric"), key: "fabric" },
    { label: t("instance.quilt"), key: "quilt" },
  ];

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        modalsContext?.closeModal();
        navigate(`/library`);
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
  const updateInstanceMutation = rspc.createMutation(
    ["instance.updateInstance"],
    {
      onSuccess() {
        modalsContext?.closeModal();
        addNotification("Instance successfully updated.");
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

  const mapTypeToColor = (
    type: McType,
    hasNoModloader: boolean | undefined
  ) => {
    return (
      <Switch>
        <Match when={type === "release"}>
          <span
            class="text-green-500"
            classList={{ "opacity-50": hasNoModloader }}
          >{`[${type}]`}</span>
        </Match>
        <Match when={type === "snapshot"}>
          <span
            class="text-yellow-500"
            classList={{ "opacity-50": hasNoModloader }}
          >{`[${type}]`}</span>
        </Match>
        <Match when={type === "old_alpha"}>
          <span
            class="text-purple-500"
            classList={{ "opacity-50": hasNoModloader }}
          >{`[${type}]`}</span>
        </Match>
        <Match when={type === "old_beta"}>
          <span
            class="text-red-500"
            classList={{ "opacity-50": hasNoModloader }}
          >{`[${type}]`}</span>
        </Match>
      </Switch>
    );
  };

  const loadIcon = (filePaths: string) => {
    fetch(`http://localhost:${port}/instance/loadIcon?path=${filePaths}`).then(
      async (img) => {
        const blob = await img.blob();
        const b64 = (await blobToBase64(blob)) as string;

        setBgPreview(
          `data:image/png;base64, ${b64.substring(b64.indexOf(",") + 1)}`
        );
      }
    );
  };

  const handleCreate = () => {
    if (!title()) {
      setError("Fields must be filled in!");
    } else {
      setError("");

      let versions: FEModdedManifestLoaderVersion[];
      if (isForge()) {
        const mcVers = forgeVersionsQuery?.data?.gameVersions[0];
        versions =
          forgeVersionsQuery?.data?.gameVersions.find(
            (v) => v.id === (mcVersion() || mcVers?.id)
          )?.loaders || [];
      } else if (isFabric()) {
        versions =
          fabricVersionsQuery?.data?.gameVersions.find(
            (v) => v.id === DUMMY_META_VERSION
          )?.loaders || [];
      } else if (isQuilt()) {
        versions =
          quiltVersionsQuery?.data?.gameVersions.find(
            (v) => v.id === DUMMY_META_VERSION
          )?.loaders || [];
      } else {
        versions = [];
      }

      trackEvent("instanceCreate", {
        loader: loader(),
        mcVersion: mcVersion() || (mappedMcVersions()?.[0]?.id as string),
      });

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
                      type_: loader() as CFFEModLoaderType,
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

  const handleUpdate = () => {
    if (instanceData()?.id) {
      setError("");

      const mcVers = forgeVersionsQuery?.data?.gameVersions[0];
      const versions =
        forgeVersionsQuery?.data?.gameVersions.find(
          (v) => v.id === (mcVersion() || mcVers?.id)
        )?.loaders || [];

      updateInstanceMutation.mutate({
        instance: parseInt((instanceData() as Instancetype).id, 10),
        use_loaded_icon: { Set: !!bgPreview() },
        name: { Set: title() },
        version: {
          Set: mcVersion() || (mappedMcVersions()?.[0]?.id as string),
        },
        modloader: {
          Set: loader()
            ? {
                type_: loader() as CFFEModLoaderType,
                version: chosenLoaderVersion() || versions[0].id,
              }
            : null,
        },
      });
    }
  };

  createEffect(() => {
    if (instanceData()?.modloader === "forge") {
      forgeVersionsQuery.refetch();
    } else if (instanceData()?.modloader === "fabric") {
      fabricVersionsQuery.refetch();
    } else if (instanceData()?.modloader === "quilt") {
      quiltVersionsQuery.refetch();
    }
  });

  return (
    <div class="flex flex-col justify-between scrollbar-hide h-full w-120 overflow-y-scroll">
      <div class="flex flex-col justify-between gap-4 h-full p-5">
        <span class="flex flex-col justify-between gap-4">
          <div class="flex gap-4 w-full">
            <div
              class="relative flex justify-center items-center bg-darkSlate-900 cursor-pointer bg-center bg-cover rounded-xl h-20 w-20"
              style={{
                "background-image": `url("${bgPreview()}")`,
              }}
              onClick={() => {
                window
                  .openFileDialog([
                    { name: "Image", extensions: ["png", "jpg", "jpeg"] },
                  ])
                  .then((files) => {
                    if (!files.filePaths[0]) return;
                    loadIcon(files.filePaths[0]);
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
                  <div class="absolute top-0 right-0 pb-2 pl-2 bg-darkSlate-700 rounded-bl-2xl">
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
            <div class="flex-1">
              <h5 class="mt-0 mb-2">
                <Trans
                  key="instance.instance_name"
                  options={{
                    defaultValue: "Instance name",
                  }}
                />
              </h5>
              <div class="flex gap-4 items-center">
                <Input
                  required
                  placeholder="New instance"
                  inputColor="bg-darkSlate-800"
                  onInput={(e) => {
                    setTitle(e.currentTarget.value);
                  }}
                  value={title() || autoGeneratedName()}
                  error={
                    error() &&
                    !title() &&
                    (t("error.missing_field_title") as string)
                  }
                />
              </div>
            </div>
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
                    if (modloader.key === "forge") {
                      forgeVersionsQuery.refetch();
                    } else if (modloader.key === "fabric") {
                      fabricVersionsQuery.refetch();
                    } else if (modloader.key === "quilt") {
                      quiltVersionsQuery.refetch();
                    }

                    setLoader(
                      !modloader.key
                        ? undefined
                        : (modloader.key as CFFEModLoaderType)
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
              <Dropdown
                disabled={Boolean(
                  ((forgeVersionsQuery.isFetching ||
                    fabricVersionsQuery.isFetching ||
                    quiltVersionsQuery.isFetching) &&
                    loader()) ||
                    mappedMcVersions().length === 0
                )}
                options={mappedMcVersions().map((v) => ({
                  label: (
                    <div
                      class="flex justify-between w-full"
                      classList={{
                        "text-darkSlate-500": Boolean(
                          !v.hasModloader && loader()
                        ),
                      }}
                    >
                      <span>{v.id}</span>
                      {mapTypeToColor(
                        v.type,
                        Boolean(!v.hasModloader && loader())
                      )}
                    </div>
                  ),
                  key: v.id,
                }))}
                bgColorClass="bg-darkSlate-800"
                containerClass="w-full"
                class="w-full"
                placement="bottom"
                value={mcVersion()}
                onChange={(l) => {
                  setMcVersion(l.key as string);

                  if (!loader) {
                    setLoaderVersions([]);
                  } else if (isForge()) {
                    const versions =
                      forgeVersionsQuery?.data?.gameVersions.find(
                        (v) => v.id === l.key
                      )?.loaders;

                    setLoaderVersions(versions || []);
                  } else if (isFabric()) {
                    const supported =
                      fabricVersionsQuery?.data?.gameVersions.find(
                        (v) => v.id === l.key
                      ) ?? false;

                    const versions =
                      supported !== false
                        ? fabricVersionsQuery?.data?.gameVersions.find(
                            (v) => v.id === DUMMY_META_VERSION
                          )?.loaders
                        : [];

                    setLoaderVersions(versions || []);
                  } else if (isQuilt()) {
                    const supported =
                      quiltVersionsQuery?.data?.gameVersions.find(
                        (v) => v.id === l.key
                      ) ?? false;

                    const versions =
                      supported !== false
                        ? quiltVersionsQuery?.data?.gameVersions.find(
                            (v) => v.id === DUMMY_META_VERSION
                          )?.loaders
                        : [];

                    setLoaderVersions(versions || []);
                  }
                }}
              />
              <div class="flex gap-4 mt-2">
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
            </div>
          </div>
          <Show when={loader()}>
            <div>
              <h5 class="mt-0 mb-2">
                <Trans
                  key="instance.instance_loader_version"
                  options={{
                    defaultValue: "Loader version",
                  }}
                />
              </h5>
              <Switch>
                <Match when={loaderVersions().length > 0}>
                  <Dropdown
                    disabled={
                      forgeVersionsQuery.isFetching ||
                      fabricVersionsQuery.isFetching ||
                      quiltVersionsQuery.isFetching ||
                      !loaderVersions()
                    }
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
                      const key = l.key as string;
                      if (key) {
                        setChosenLoaderVersion(key);
                      }
                    }}
                  />
                </Match>
                <Match when={loaderVersions().length === 0}>
                  <Dropdown
                    disabled={true}
                    options={[{ label: "No elements", key: "none" }]}
                    bgColorClass="bg-darkSlate-800"
                    containerClass="w-full"
                    class="w-full"
                    value={"none"}
                    placement="bottom"
                  />
                </Match>
              </Switch>
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
            disabled={Boolean(
              (loaderVersions().length === 0 && loader()) ||
                mappedMcVersions().length === 0
            )}
            type="primary"
            style={{ width: "100%", "max-width": "200px" }}
            onClick={() => {
              if (instanceData()) handleUpdate();
              else handleCreate();
            }}
          >
            <Switch>
              <Match when={!instanceData()}>
                <Trans
                  key="instance.instance_modal_instance_creation_create"
                  options={{
                    defaultValue: "Create",
                  }}
                />
              </Match>
              <Match when={instanceData()}>
                <Trans
                  key="instance.instance_modal_instance_update"
                  options={{
                    defaultValue: "Update",
                  }}
                />
              </Match>
            </Switch>
          </Button>
        </div>
      </div>
    </div>
  );
};

export default Custom;
