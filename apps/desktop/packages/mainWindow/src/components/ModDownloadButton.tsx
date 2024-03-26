import { rspc } from "@/utils/rspcClient";
import { Mod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Progressbar, Spinner, Tooltip } from "@gd/ui";
import { Match, Show, Switch, createEffect, createSignal } from "solid-js";

type ModDownloadButtonProps = {
  projectId: number | string | undefined;
  fileId?: number | string;
  instanceId: number | null | undefined;
  size: "small" | "medium" | "large";
  isCurseforge: boolean;
  instanceMods: Mod[] | undefined;
  instanceLocked: boolean | undefined;
};

const ModDownloadButton = (props: ModDownloadButtonProps) => {
  const [loading, setLoading] = createSignal(false);
  const [taskId, setTaskId] = createSignal<number | null>(null);
  const [progress, setProgress] = createSignal<number | null>(null);

  const installLatestModMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installLatestMod",
    onSuccess(taskId) {
      setTaskId(taskId);
    },
    onMutate() {
      setLoading(true);
    }
  }));

  createEffect(async () => {
    if (taskId() !== null) {
      const task = rspc.createQuery(() => ({
        queryKey: ["vtask.getTask", taskId()]
      }));

      createEffect(() => {
        if (task?.data?.progress.type === "Known") {
          setProgress(Math.round(task?.data?.progress.value * 100));
        }
      });
    }
  });

  const installedMod = () =>
    props.instanceMods?.find(
      (mod) =>
        (props.isCurseforge
          ? mod.curseforge?.project_id
          : mod.modrinth?.project_id
        )?.toString() === props.projectId?.toString()
    );

  const installModMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installMod"
  }));

  createEffect(() => {
    if (installModMutation.isPending) {
      setLoading(true);
    }

    if (installModMutation.isSuccess) {
      setTaskId(installModMutation.data);
    }
  });

  const latestModInstallObj = () => {
    return props.isCurseforge
      ? {
          Curseforge: parseInt(props.projectId!.toString(), 10)
        }
      : {
          Modrinth: props.projectId!.toString()
        };
  };

  const modInstallObj = () => {
    return props.isCurseforge
      ? {
          Curseforge: {
            project_id: parseInt(props.projectId!.toString(), 10),
            file_id: parseInt(props.fileId!.toString(), 10)
          }
        }
      : {
          Modrinth: {
            project_id: props.projectId!.toString(),
            version_id: props.fileId!.toString()
          }
        };
  };

  const isInstalled = () => {
    if (!installedMod()) return false;

    if (!props.fileId) {
      return !!installedMod();
    } else {
      if (props.isCurseforge) {
        return (
          installedMod()?.curseforge?.file_id ===
          parseInt(props.fileId.toString(), 10)
        );
      } else {
        return installedMod()?.modrinth?.version_id === props.fileId.toString();
      }
    }
  };

  const handleDownload = async () => {
    if (!props.instanceId || isInstalled()) return;

    if (!props.fileId) {
      await installLatestModMutation.mutateAsync({
        instance_id: props.instanceId,
        mod_source: latestModInstallObj()
      });
    } else {
      const replacesMod = installedMod()?.id || null;

      await installModMutation.mutateAsync({
        mod_source: modInstallObj(),
        instance_id: props.instanceId,
        install_deps: !replacesMod,
        replaces_mod: replacesMod
      });
    }
  };

  createEffect(() => {
    if (isInstalled()) {
      setLoading(false);
      setTaskId(null);
      setProgress(null);
    }
  });

  return (
    <Tooltip
      content={
        props.instanceLocked ? (
          <Trans key="instance.locked_cannot_apply_changes" />
        ) : null
      }
    >
      <Button
        uppercase
        size={props.size}
        variant={isInstalled() ? "green" : "primary"}
        disabled={!props.instanceId || (props.instanceLocked && !isInstalled())}
        onClick={handleDownload}
      >
        <Show when={loading()}>
          <Spinner />
          <div
            class="duration-100 ease-in-out transition-width"
            classList={{
              "w-0": progress() === null,
              "w-14": progress() !== null
            }}
          >
            <Progressbar color="bg-white" percentage={progress()!} />
          </div>
        </Show>
        <Show when={!loading()}>
          <Switch>
            <Match when={!props.instanceId || isNaN(props.instanceId)}>
              <Trans key="instance.no_instance_selected" />
            </Match>
            <Match when={isInstalled()}>
              <Trans key="mod.downloaded" />
            </Match>
            <Match when={props.instanceLocked}>
              <Trans key="instance.instance_locked" />
            </Match>
            <Match when={!props.instanceLocked && !props.fileId}>
              <Trans key="instance.download_latest" />
            </Match>
            <Match
              when={
                !props.instanceLocked &&
                props.fileId &&
                installedMod() &&
                !isInstalled()
              }
            >
              <Trans key="instance.switch_version" />
            </Match>
            <Match when={!props.instanceLocked && props.fileId}>
              <Trans key="instance.download_version" />
            </Match>
          </Switch>
        </Show>
      </Button>
    </Tooltip>
  );
};

export default ModDownloadButton;
