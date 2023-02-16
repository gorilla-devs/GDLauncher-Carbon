import { ModloaderType, getModloaderIcon } from "@/utils/sidebar";
import { Checkbox, Switch } from "@gd/ui";

type ModType = {
  title: string;
  enabled: boolean;
  modloader: ModloaderType;
  mcversion: string;
  modloaderVersion: string;
};

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
            <div class="h-10 w-10 bg-green-500 rounded-xl" />
            <div class="flex flex-col">
              {props.mod.title}
              <div class="flex gap-2">
                <img
                  class="w-4 h-4"
                  src={getModloaderIcon(props.mod.modloader)}
                />
                <p class="m-0 text-shade-5 text-sm">
                  {`${props.mod.modloader} ${props.mod.mcversion}`}
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
