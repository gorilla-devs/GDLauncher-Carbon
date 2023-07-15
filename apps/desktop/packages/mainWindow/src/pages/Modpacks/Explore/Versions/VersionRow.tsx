import {
  FEFile,
  FEMod,
  FEModrinthProject,
  FEModrinthVersion,
  FETask,
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";
import { Spinner, createNotification } from "@gd/ui";
import { Match, Switch, createEffect, createSignal } from "solid-js";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";

type Props = {
  modVersion: FEModrinthVersion | FEFile;
  project: FEMod | FEModrinthProject | undefined;
  isCurseForge?: boolean;
};

const VersionRow = (props: Props) => {
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);
  const dismissTaskMutation = rspc.createMutation(["vtask.dismissTask"]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const [taskId, setTaskId] = createSignal<undefined | number>(undefined);
  const [task, setTask] = createSignal<CreateQueryResult<
    FETask | null,
    RSPCError
  > | null>(null);

  createEffect(() => {
    if (taskId() !== undefined) {
      // eslint-disable-next-line solid/reactivity
      setTask(rspc.createQuery(() => ["vtask.getTask", taskId() as number]));
    }
  });

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess(_data, taskId) {
        setTaskId(taskId);
        addNotification("Instance successfully created.");
      },
      onError() {
        if (taskId()) dismissTaskMutation.mutate(taskId() as number);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        if (taskId()) dismissTaskMutation.mutate(taskId() as number);
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

  // const getUrl = () => {
  //   if (props.isCurseForge) {
  //     return (props.modVersion as FEFile).displayName;
  //   }
  //   return (props.modVersion as FEModrinthVersion).;
  // };
  const getName = () => {
    if (props.isCurseForge) {
      return (props.modVersion as FEFile).displayName;
    }
    return (props.modVersion as FEModrinthVersion).name;
  };

  const getDate = () => {
    if (props.isCurseForge) {
      return (props.modVersion as FEFile).fileDate;
    }
    return (props.modVersion as FEModrinthVersion).date_published;
  };

  const getLastGameVersion = () => {
    if (props.isCurseForge) {
      return (props.modVersion as FEFile).gameVersions[0];
    }
    return (props.modVersion as FEModrinthVersion).version_number;
  };

  const getReleaseType = () => {
    if (props.isCurseForge) {
      return (props.modVersion as FEFile).releaseType;
    }
    return (props.modVersion as FEModrinthVersion).version_type;
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
                "text-green-500": getReleaseType() === "stable",
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
          const icon = props.isCurseForge
            ? (props.project as FEMod).logo.url
            : (props.project as FEModrinthProject).icon_url;

          const modpack = props.isCurseForge
            ? {
                Curseforge: {
                  file_id: (props.modVersion as FEFile).id,
                  project_id: (props.modVersion as FEFile).modId,
                },
              }
            : {
                Modrinth: {
                  project_id: (props.modVersion as FEModrinthVersion)
                    .project_id,
                  version_id: (props.modVersion as FEModrinthVersion).id,
                },
              };

          if (icon) {
            loadIconMutation.mutate(icon);
          }
          createInstanceMutation.mutate({
            group: defaultGroup.data || 1,
            use_loaded_icon: true,
            notes: "",
            name: props.isCurseForge
              ? (props.modVersion as FEFile).displayName
              : (props.project as FEModrinthProject).title,
            version: {
              Modpack: modpack,
            },
          });
        }}
      >
        <Switch>
          <Match when={task()}>
            <Trans
              key="modpack.version_downloading"
              options={{
                defaultValue: "Downloading...",
              }}
            />
            <Spinner class="w-5 h-5" />
          </Match>
          <Match when={!task()}>
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
