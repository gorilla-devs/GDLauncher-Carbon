import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, Spinner, Tooltip } from "@gd/ui";
import {
  JSX,
  Match,
  Show,
  Switch,
  children,
  createEffect,
  createSignal
} from "solid-js";

const MaybeTooltip = (props: {
  children: JSX.Element;
  showTooltip?: boolean;
}) => {
  const c = children(() => props.children);

  return (
    <Switch>
      <Match when={!props.showTooltip}>{c()}</Match>
      <Match when={props.showTooltip}>
        <Tooltip content={<Trans key="instance.locked_cannot_apply_changes" />}>
          {c()}
        </Tooltip>
      </Match>
    </Switch>
  );
};

type ModDownloadButtonProps = {
  projectId: number | string | undefined;
  fileId?: number | string;
  instanceId: number | null | undefined;
  size: "small" | "medium" | "large";
  isCurseforge: boolean;
};

const ModDownloadButton = (props: ModDownloadButtonProps) => {
  const [loading, setLoading] = createSignal(false);

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    props.instanceId || null
  ]);

  const instanceMods = rspc.createQuery(() => [
    "instance.getInstanceMods",
    props.instanceId || null
  ]);

  const installLatestModMutation = rspc.createMutation(
    ["instance.installLatestMod"],
    {
      onMutate() {
        setLoading(true);
      }
    }
  );

  const installedMod = () =>
    instanceMods.data?.find(
      (mod) =>
        (props.isCurseforge
          ? mod.curseforge?.project_id
          : mod.modrinth?.project_id
        )?.toString() === props.projectId?.toString()
    );

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
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

  const handleDownload = () => {
    if (!props.instanceId || isInstalled()) return;

    if (!props.fileId) {
      installLatestModMutation.mutate({
        instance_id: props.instanceId,
        mod_source: latestModInstallObj()
      });
    } else {
      const replacesMod = installedMod()?.id || null;

      installModMutation.mutate({
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
    }
  });

  return (
    <MaybeTooltip showTooltip={instanceDetails.data?.modpack?.locked}>
      <Button
        uppercase
        size={props.size}
        variant={isInstalled() ? "green" : "primary"}
        disabled={
          !props.instanceId ||
          (instanceDetails.data?.modpack?.locked && !isInstalled())
        }
        onClick={handleDownload}
      >
        <Show when={loading()}>
          <Spinner />
        </Show>
        <Show when={!loading()}>
          <Switch>
            <Match when={!props.instanceId || isNaN(props.instanceId)}>
              <Trans key="instance.no_instance_selected" />
            </Match>
            <Match when={isInstalled()}>
              <Trans key="mod.downloaded" />
            </Match>
            <Match when={instanceDetails.data?.modpack?.locked}>
              <Trans key="instance.instance_locked" />
            </Match>
            <Match
              when={!instanceDetails.data?.modpack?.locked && !props.fileId}
            >
              <Trans key="instance.download_latest" />
            </Match>
            <Match
              when={
                !instanceDetails.data?.modpack?.locked &&
                props.fileId &&
                installedMod() &&
                !isInstalled()
              }
            >
              <Trans key="instance.switch_version" />
            </Match>
            <Match
              when={!instanceDetails.data?.modpack?.locked && props.fileId}
            >
              <Trans key="instance.download_version" />
            </Match>
          </Switch>
        </Show>
      </Button>
    </MaybeTooltip>
  );
};

export default ModDownloadButton;
