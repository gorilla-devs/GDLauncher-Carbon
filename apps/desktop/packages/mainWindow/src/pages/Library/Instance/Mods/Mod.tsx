import { Checkbox, Switch } from "@gd/ui";
import forgeIcon from "/assets/images/icons/forge.png";
import vanillaIcon from "/assets/images/icons/vanilla.png";

type Modloader = "forge" | "vanilla" | "fabric";
interface ModType {
  title: string;
  enabled: boolean;
  modloader: Modloader;
  mcversion: string;
  modloaderVersion: string;
}

interface Props {
  mod: ModType;
}

const getIcon = (modloader: Modloader) => {
  switch (modloader) {
    case "vanilla":
      return vanillaIcon;
    case "forge":
      return forgeIcon;
    default:
      return vanillaIcon;
  }
};

const Mod = (props: Props) => {
  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 justify-between items-center w-full">
        <div class="flex gap-4 justify-between items-center">
          <Checkbox checked={true} disabled={false} />
          <div class="flex items-center gap-2">
            <div class="bg-green-500 h-10 w-10 rounded-xl" />
            <div class="flex flex-col">
              {props.mod.title}
              <div class="flex gap-2">
                <img class="w-4 h-4" src={getIcon(props.mod.modloader)} />
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
