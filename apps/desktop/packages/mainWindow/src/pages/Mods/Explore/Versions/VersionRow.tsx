import {
  CFFEFile,
  CFFEMod,
  MRFEProject,
  MRFEVersion
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import { Spinner } from "@gd/ui";
import { Match, Switch, createEffect, createSignal } from "solid-js";

type Props = {
  modVersion: MRFEVersion | CFFEFile;
  project: CFFEMod | MRFEProject | undefined;
  isCurseforge?: boolean;
  disabled?: boolean;
  instanceId: number | null;
  installedFile:
    | {
        id: string;
        remote_id: string | number;
      }
    | undefined;
};

const VersionRow = (props: Props) => {
  const [loading, setLoading] = createSignal(false);
  const [taskId, setTaskId] = createSignal<number | null>(null);

  const task = rspc.createQuery(() => ["vtask.getTask", taskId()]);

  createEffect(() => {
    if (
      taskId() !== null &&
      taskId() !== undefined &&
      task.data !== undefined &&
      task.data !== null
    ) {
      setLoading(false);
      setTaskId(null);
    }
  });

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

  const instanceId = () => props?.instanceId;

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    },
    onSuccess(data) {
      setTaskId(data);
    }
  });

  const isInstalled = () => {
    if (props.isCurseforge) {
      return (
        props.installedFile?.remote_id ===
          (props.modVersion as CFFEFile).id.toString() &&
        props.installedFile !== null
      );
    }

    return (
      props.installedFile?.remote_id === (props.modVersion as MRFEVersion).id &&
      props.installedFile?.remote_id !== null
    );
  };

  return (
    <div class="group flex justify-between items-center py-2 rounded-md px-2 hover:bg-darkSlate-900">
      <div class="flex flex-col">
        <h4 class="m-0 font-medium group-hover:text-lightSlate-200">
          {getName().replaceAll(".zip", "")}
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
                "text-red-500": getReleaseType() === "alpha"
              }}
            >
              {getReleaseType()}
            </span>
          </div>
        </div>
      </div>
      <span
        class="flex gap-2 select-none items-center"
        classList={{
          "cursor-pointer text-lightGray-800 group-hover:text-lightSlate-50 group-hover:text-lg transition transition-all duration-75 ease-in-out":
            !props.disabled,
          "cursor-not-allowed text-lightGray-800": props.disabled
        }}
        onClick={() => {
          if (props.disabled || !instanceId()) return;
          const mod = props.isCurseforge
            ? {
                Curseforge: {
                  file_id: (props.modVersion as CFFEFile).id,
                  project_id: (props.modVersion as CFFEFile).modId
                }
              }
            : {
                Modrinth: {
                  project_id: (props.modVersion as MRFEVersion).project_id,
                  version_id: (props.modVersion as MRFEVersion).id
                }
              };

          console.log("INSTALLED MOD", props.installedFile?.id || null);

          installModMutation.mutate({
            mod_source: mod,
            instance_id: instanceId()!,
            install_deps: !isInstalled(),
            replaces_mod: isInstalled() ? props.installedFile?.id || null : null
          });
        }}
      >
        <Switch>
          <Match when={loading()}>
            <Trans key="modpack.version_downloading" />
            <Spinner class="w-5 h-5" />
          </Match>
          <Match when={!loading() && !isInstalled()}>
            <Trans key="modpack.version_download" />
            <div class="i-ri:download-2-line" />
          </Match>
          <Match when={!loading() && isInstalled()}>
            <Trans key="modpack.version_installed" />
          </Match>
        </Switch>
      </span>
    </div>
  );
};

export default VersionRow;
