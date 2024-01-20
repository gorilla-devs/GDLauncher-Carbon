import RowContainer, { Props } from "@/components/Browser/RowContainer";
import { rspc } from "@/utils/rspcClient";
import { ModSource } from "@gd/core_module/bindings";
import { createEffect, createSignal } from "solid-js";

const VersionRow = (props: Props) => {
  const [loading, setLoading] = createSignal(false);

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    }
  });

  const onPrimaryAction = () => {
    if (!props.instanceId) return;
    const mod: ModSource = props.isCurseforge
      ? {
          Curseforge: {
            project_id: parseInt(props.modVersion.id, 10),
            file_id: parseInt(props.modVersion.fileId, 10)
          }
        }
      : {
          Modrinth: {
            project_id: props.modVersion.id,
            version_id: props.modVersion.fileId
          }
        };

    installModMutation.mutate({
      mod_source: mod,
      instance_id: props.instanceId!,
      install_deps: !props.installedFile?.id,
      replaces_mod: props.installedFile?.id || null
    });
  };

  const isInstalled = () => {
    return (
      props.installedFile?.remoteId.toString() ===
        props.modVersion?.fileId.toString() &&
      props.installedFile?.remoteId !== null
    );
  };

  createEffect(() => {
    if (isInstalled()) {
      setLoading(false);
    }
  });

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
