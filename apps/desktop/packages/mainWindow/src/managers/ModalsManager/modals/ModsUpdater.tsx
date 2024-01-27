import { Progressbar, createNotification } from "@gd/ui";
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Trans } from "@gd/i18n";

import { rspc } from "@/utils/rspcClient";
import { createSignal, onCleanup } from "solid-js";
import { Mod } from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";

type Props = {
  instanceId: number;
  mods: Mod[];
};

const AppUpdate = (props: ModalProps) => {
  const data: () => Props = () => props?.data;
  const addNotification = createNotification();
  const modalsContext = useModal();
  const [modsUpdated, setModsUpdated] = createSignal(0);
  const [isDestroyed, setIsDestroyed] = createSignal(false);

  onCleanup(() => {
    setIsDestroyed(true);
  });

  const updateModMutation = rspc.createMutation(["instance.updateMod"], {
    onError: (err) => {
      console.error(err);
      addNotification(`Error updating mod: ${err.cause?.message}`, "error");
    }
  });

  const updateMods = async () => {
    for (const modId of data().mods) {
      try {
        await updateModMutation.mutateAsync({
          instance_id: data().instanceId,
          mod_id: modId.id
        });
      } catch (err) {
        console.error(err);
        addNotification(
          `Error updating mod: ${(err as RSPCError).cause?.message}`,
          "error"
        );
      } finally {
        setModsUpdated((prev) => prev + 1);
      }

      if (isDestroyed()) return;
    }

    addNotification("Mods updated successfully!", "success");
    modalsContext?.closeModal();
  };

  const currentModName = () => {
    const mod = data().mods[modsUpdated()];

    if (!mod) return "";

    return mod.curseforge?.name || mod.modrinth?.title || mod.filename;
  };

  updateMods();

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="flex flex-col overflow-hidden min-h-60 w-160">
        <div class="text-xl flex items-center">
          <div class="w-140">
            <Trans
              key="mods_updater.updating_mods_count"
              options={{
                mods: modsUpdated(),
                total: data().mods.length
              }}
            />
          </div>
          <Progressbar
            percentage={(modsUpdated() / data().mods.length) * 100}
          />
        </div>
        <div class="flex flex-col items-center text-xl mt-20">
          <Trans
            key="mods_updater.updating_mod_text"
            options={{
              mod_name: currentModName()
            }}
          />
          <i class="i-ri:loader-4-line animate-spin text-6xl mt-10" />
        </div>
      </div>
    </ModalLayout>
  );
};

export default AppUpdate;
