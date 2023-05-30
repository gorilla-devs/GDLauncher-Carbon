import { lastInstanceOpened } from "@/utils/routes";
import { rspc } from "@/utils/rspcClient";
import { getModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import { Checkbox, Switch } from "@gd/ui";

type Props = {
  mod: ModType;
};

const Mod = (props: Props) => {
  const enableModMutation = rspc.createMutation(["instance.enableMod"]);
  const disableModMutation = rspc.createMutation(["instance.disableMod"]);

  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 justify-between items-center w-full">
        <div class="flex gap-4 justify-between items-center">
          <Checkbox checked={true} disabled={false} />
          <div class="flex items-center gap-2">
            <div class="h-10 w-10 rounded-xl bg-green-500" />
            <div class="flex flex-col">
              {props.mod.filename}
              <div class="flex gap-2">
                <img
                  class="w-4 h-4"
                  src={getModloaderIcon(props.mod.modloader)}
                />
                <img class="w-4 h-4" src={getModloaderIcon("Forge")} />
                <p class="m-0 text-darkSlate-500 text-sm">
                  {`${props.mod.modloader} ${props.mod.metadata.version}`}
                </p>
                <p class="m-0 text-darkSlate-500 text-sm">
                  {`${"forge"} ${"2.1.3"}`}
                </p>
              </div>
            </div>
          </div>
        </div>
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
      </div>
    </div>
  );
};

export default Mod;
