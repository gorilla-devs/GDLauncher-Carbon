import { lastInstanceOpened } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { getModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import { Checkbox, Switch } from "@gd/ui";
import { useParams } from "@solidjs/router";
import { SetStoreFunction } from "solid-js/store";
import { For } from "solid-js";

type Props = {
  mod: ModType;
  setSelectedMods: SetStoreFunction<{
    [id: string]: boolean;
  }>;
  selectedMods: {
    [id: string]: boolean;
  };
};

const Mod = (props: Props) => {
  const enableModMutation = rspc.createMutation(["instance.enableMod"]);
  const disableModMutation = rspc.createMutation(["instance.disableMod"]);
  const deleteModMutation = rspc.createMutation(["instance.deleteMod"]);
  const params = useParams();

  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 justify-between items-center w-full">
        <div class="flex gap-4 justify-between items-center">
          <Checkbox
            checked={props.selectedMods[props.mod.id]}
            onChange={(e) => {
              props.setSelectedMods(props.mod.id, e);
            }}
          />
          <div class="flex items-center gap-2">
            <div class="h-10 w-10 rounded-xl bg-green-500" />
            <div class="flex flex-col">
              {props.mod.metadata.name}
              <div class="flex gap-2">
                <For each={props.mod.modloaders}>
                  {(modloader, _) => (
                    <span class="flex gap-2 justify-center items-center">
                      <img class="w-4 h-4" src={getModloaderIcon(modloader)} />
                      <p class="m-0 text-darkSlate-500 text-sm">
                        {`${modloader}`}
                      </p>
                    </span>
                  )}
                </For>
                <p class="m-0 text-darkSlate-500 text-sm">
                  {`${props.mod.metadata.version}`}
                </p>
              </div>
            </div>
          </div>
        </div>
        <span class="flex gap-4 justify-center items-center">
          {/* //TODO: ADD CONFIRMATION MODAL */}
          <div
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer transition-color i-ri:delete-bin-2-fill hover:text-red-500"
            onClick={() => {
              deleteModMutation.mutate({
                instance_id: parseInt(params.id, 10),
                mod_id: props.mod.id,
              });
            }}
          />
          <Switch
            checked={props.mod.enabled}
            onChange={(e) => {
              if (e.target.checked) {
                enableModMutation.mutate({
                  instance_id: parseInt(lastInstanceOpened(), 10),
                  mod_id: props.mod.id,
                });
              } else {
                disableModMutation.mutate({
                  instance_id: parseInt(lastInstanceOpened(), 10),
                  mod_id: props.mod.id,
                });
              }
            }}
          />
        </span>
      </div>
    </div>
  );
};

export default Mod;
