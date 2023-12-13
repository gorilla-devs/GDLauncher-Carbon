import RowContainer, { Props } from "@/components/Browser/RowContainer";
import { rspc } from "@/utils/rspcClient";
import { CFFEFile, MRFEVersion } from "@gd/core_module/bindings";
import { createEffect, createSignal } from "solid-js";

const VersionRow = (props: Props) => {
  const [loading, setLoading] = createSignal(false);
  const [taskId, setTaskId] = createSignal<number | null>(null);

  const task = rspc.createQuery(() => ["vtask.getTask", taskId()]);

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    },
    onSuccess(data) {
      setTaskId(data);
    }
  });

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

  const onPrimaryAction = () => {
    if (!props.instanceId) return;
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

    installModMutation.mutate({
      mod_source: mod,
      instance_id: props.instanceId!,
      install_deps: !props.instanceId,
      replaces_mod: props.installedFile?.id || null
    });
  };

  const isInstalled = () => {
    if (props.isCurseforge) {
      return (
        props.installedFile?.remoteId ===
          (props.modVersion as CFFEFile).id.toString() &&
        props.installedFile !== null
      );
    }

    return (
      props.installedFile?.remoteId === (props.modVersion as MRFEVersion).id &&
      props.installedFile?.remoteId !== null
    );
  };

  return (
    <RowContainer
      {...props}
      loading={loading()}
      disabled={!props.instanceId}
      onPrimaryAction={onPrimaryAction}
      isInstalled={isInstalled()}
    />
  );
};

export default VersionRow;
