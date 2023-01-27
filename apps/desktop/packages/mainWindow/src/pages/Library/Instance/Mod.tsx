import { Checkbox } from "@gd/ui";

interface IMod {
  title: string;
  enabled: boolean;
  modloader: string;
  mcversion: string;
  modloaderVersion: string;
}

interface Props {
  mod: IMod;
}

const Mod = (props: Props) => {
  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 items-center">
        <Checkbox checked={true} disabled={false} />
        <div class="flex items-center gap-2">
          <div class="bg-green-500 h-10 w-10 rounded-xl" />
          {props.mod.title}
        </div>
      </div>
    </div>
  );
};

export default Mod;
