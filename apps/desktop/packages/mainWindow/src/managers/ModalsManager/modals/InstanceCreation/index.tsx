import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans } from "@gd/i18n";
import Custom from "./Custom";
import Import from "./Import";

const InstanceCreation = (props: ModalProps) => {
  const [t] = useTransContext();
  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    MappedMcVersions[]
  >([]);

  const instanceData = () => props.data as Instancetype | undefined;

  const [error, setError] = createSignal("");
  const [bgPreview, setBgPreview] = createSignal<string | null>(
    instanceData()?.img || null
  );
  const [loader, setLoader] = createSignal<ModLoaderType | undefined>(
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

  createEffect(() => {
    if (forgeVersionsQuery.data && loader() === "Forge") {
      const versions = forgeVersionsQuery?.data?.gameVersions.find(
        (v) => v.id === (mcVersion() || (mappedMcVersions()?.[0]?.id as string))
      )?.loaders;

      setLoaderVersions(versions || []);
    } else if (!loader()) {
      setLoaderVersions([]);
    }
  });

  createEffect(() => {
    if (fabricVersionsQuery.data && loader() === "Fabric") {
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
    if (quiltVersionsQuery.data && loader() === "Quilt") {
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

    if (loader() === "forge") setMappedMcVersions(forgeMappedVersions);
    else if (loader() === "fabric") setMappedMcVersions(fabricMappedVersions);
    else if (loader() === "quilt") setMappedMcVersions(quiltMappedVersions);
    else setMappedMcVersions(filteredData);
  });

  const autoGeneratedName = () =>
    `${loader() || "Vanilla"} ${
      mcVersion() || (mappedMcVersions()?.[0]?.id as string)
    }`;

  createEffect(() => {
    setTitle(autoGeneratedName());
  });

  const modloaders = [
    { label: t("instance.vanilla"), key: undefined },
    { label: t("instance.forge"), key: "Forge" },
    { label: t("instance.fabric"), key: "Fabric" },
    { label: t("instance.quilt"), key: "Quilt" },
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
      if (loader() == "Forge") {
        const mcVers = forgeVersionsQuery?.data?.gameVersions[0];
        versions =
          forgeVersionsQuery?.data?.gameVersions.find(
            (v) => v.id === (mcVersion() || mcVers?.id)
          )?.loaders || [];
      } else if (loader() == "Fabric") {
        versions =
          fabricVersionsQuery?.data?.gameVersions.find(
            (v) => v.id === DUMMY_META_VERSION
          )?.loaders || [];
      } else if (loader() == "Quilt") {
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
                type_: loader() as ModLoaderType,
                version: chosenLoaderVersion() || versions[0].id,
              }
            : null,
        },
      });
    }
  };

  createEffect(() => {
    if (instanceData()?.modloader === "Forge") {
      forgeVersionsQuery.refetch();
    }
  });

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
    >
      <div class="flex flex-col justify-between scrollbar-hide w-120 overflow-y-scroll h-136">
        <Tabs variant="block" paddingY="py-2">
          <TabList>
            <Tab>
              <Trans key="instance.instance_creation_custom_tab" />
            </Tab>
            <Tab>
              <Trans key="instance.instance_import_tab" />
            </Tab>
          </TabList>
          <TabPanel>
            <Custom data={props.data} />
          </TabPanel>
          <TabPanel>
            <Import />
          </TabPanel>
        </Tabs>
      </div>
    </ModalLayout>
  );
};

export default InstanceCreation;
