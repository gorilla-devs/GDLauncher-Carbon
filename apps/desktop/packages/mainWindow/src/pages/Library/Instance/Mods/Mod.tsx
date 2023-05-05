import { getModloaderIcon } from "@/utils/sidebar";
import { Mod as ModType } from "@gd/core_module/bindings";
import { Checkbox, Switch } from "@gd/ui";

type Props = {
  mod: ModType;
};

const Mod = (props: Props) => {
  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 justify-between items-center w-full">
        <div class="flex gap-4 justify-between items-center">
          <Checkbox checked={true} disabled={false} />
          <div class="flex items-center gap-2">
            <div class="h-10 w-10 rounded-xl bg-green-500" />
            <div class="flex flex-col">
              {props.mod.name}
              <div class="flex gap-2">
                {/* <img
                  class="w-4 h-4"
                  src={getModloaderIcon(props.mod.modloader)}
                /> */}
                <img class="w-4 h-4" src={getModloaderIcon("Forge")} />
                {/* <p class="m-0 text-darkSlate-500 text-sm">
                  {`${props.mod.modloader} ${props.mod.mcversion}`}
                </p> */}
                <p class="m-0 text-darkSlate-500 text-sm">
                  {`${"forge"} ${"2.1.3"}`}
                </p>
              </div>
            </div>
          </div>
        </div>
        <Switch />
      </div>
    </div>
  );
};

export default Mod;
