import {
  CFFEFile,
  CFFEMod,
  MRFEProject,
  MRFEVersion,
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";
import { Spinner, createNotification } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";

type Props = {
  modVersion: MRFEVersion | CFFEFile;
  project: CFFEMod | MRFEProject | undefined;
  isCurseforge?: boolean;
};

const VersionRow = (props: Props) => {
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const [loading, setLoading] = createSignal(false);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess(_data) {
        setLoading(true);
        addNotification("Instance successfully created.");
      },
      onError() {
        setLoading(false);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        setLoading(false);
        navigate(`/library`);
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
        addNotification("Error while downloading the modpack.", "error");
      },
    }
  );

  const getDate = () => {
    if (props.isCurseforge) {
      return (props.modVersion as CFFEFile).fileDate;
    }
    return (props.modVersion as MRFEVersion).date_published;
  };

  const getLastGameVersion = () => {
    if (props.isCurseforge) {
      return (props.modVersion as CFFEFile).gameVersions[0];
    }
    return (props.modVersion as MRFEVersion).game_versions[0];
  };

  const getName = () => {
    if (props.isCurseforge) {
      return (props.modVersion as CFFEFile).displayName;
    }
    return `${(props.project as MRFEProject).title} ${getLastGameVersion()}`;
  };

  const getReleaseType = () => {
    if (props.isCurseforge) {
      return (props.modVersion as CFFEFile).releaseType;
    }
    return (props.modVersion as MRFEVersion).version_type;
  };

  return (
    <div class="group flex justify-between items-center py-2 rounded-md px-2 hover:bg-darkSlate-900">
      <div class="flex flex-col">
        <h4 class="m-0 font-medium group-hover:text-lightSlate-200">
          {getName()}
        </h4>
        <div class="flex justify-between items-center">
          <div class="flex justify-between">
            <div class="flex justify-between text-sm divide-darkSlate-500 text-lightGray-800 divide-x-1">
              <span class="pr-3">{getLastGameVersion()}</span>
              <span class="px-3">
                {format(new Date(getDate()), "dd-MM-yyyy")}
              </span>
            </div>
            <span
              class="pl-3"
              classList={{
                "text-green-500":
                  getReleaseType() === "stable" ||
                  getReleaseType() === "release",
                "text-yellow-500": getReleaseType() === "beta",
                "text-red-500": getReleaseType() === "alpha",
              }}
            >
              {getReleaseType()}
            </span>
          </div>
        </div>
      </div>
      <span
        class="flex gap-2 text-lightGray-800 cursor-pointer select-none group-hover:text-lightSlate-50"
        onClick={() => {
          const icon = props.isCurseforge
            ? (props.project as CFFEMod).logo.url
            : (props.project as MRFEProject).icon_url;

          const modpack = props.isCurseforge
            ? {
                Curseforge: {
                  file_id: (props.modVersion as CFFEFile).id,
                  project_id: (props.modVersion as CFFEFile).modId,
                },
              }
            : {
                Modrinth: {
                  project_id: (props.modVersion as MRFEVersion).project_id,
                  version_id: (props.modVersion as MRFEVersion).id,
                },
              };

          if (icon) {
            loadIconMutation.mutate(icon);
          }
          setLoading(true);
          console.log("props.modVersion", props.modVersion);
          createInstanceMutation.mutate({
            group: defaultGroup.data || 1,
            use_loaded_icon: true,
            notes: "",
            name: props.isCurseforge
              ? (props.modVersion as CFFEFile).displayName
              : (props.project as MRFEProject).title,
            version: {
              Modpack: modpack,
            },
          });
        }}
      >
        <Switch fallback={<div>Loading...</div>}>
          <Match when={loading()}>
            <Trans
              key="modpack.version_downloading"
              options={{
                defaultValue: "Downloading...",
              }}
            />
            <Spinner class="w-5 h-5" />
          </Match>
          <Match when={!loading()}>
            <Trans
              key="modpack.version_download"
              options={{
                defaultValue: "Download version",
              }}
            />
            <div class="i-ri:download-2-line" />
          </Match>
        </Switch>
      </span>
    </div>
  );
};

export default VersionRow;
