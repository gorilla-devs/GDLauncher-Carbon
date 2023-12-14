import RowContainer, { Props } from "@/components/Browser/RowContainer";
import { rspc } from "@/utils/rspcClient";
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
    const mod = props.isCurseforge
      ? {
          Curseforge: {
            project_id: props.modVersion.id,
            file_id: props.modVersion.fileId
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
      install_deps: !props.instanceId,
      replaces_mod: props.installedFile?.id || null
    });
  };

  const isInstalled = () => {
    if (props.isCurseforge) {
      return (
        props.installedFile?.remoteId === props.modVersion?.id &&
        props.installedFile !== null
      );
    }

    return (
      props.installedFile?.remoteId === props.modVersion?.id &&
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
