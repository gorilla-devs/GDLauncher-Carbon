import { Trans } from "@gd/i18n";
import { Show } from "solid-js";

type TypeProps = {
  title: string;
  mcversion: string;
  modloader: string;
  date: string;
  stable: string;
  isActive: boolean;
};

type Props = {
  version: TypeProps;
};

const getColor = (stable: string) => {
  switch (stable) {
    case "stable":
      return "text-green";
    case "beta":
      return "text-yellow";
    case "alpha":
      return "text-red";
    default:
      return "text-green";
  }
};

const Active = () => {
  return (
    <div class="text-green flex items-center gap-2 cursor-pointer">
      <Trans
        key="active_version"
        options={{
          defaultValue: "Active",
        }}
      />
      <div class="i-ri:check-fill text-green text-2xl" />
    </div>
  );
};

const Version = (props: Props) => {
  return (
    <div class="w-full h-14 flex items-center py-2 box-border">
      <div class="flex gap-4 justify-between items-center w-full">
        <div class="flex gap-4 justify-between items-center">
          <div class="flex items-center gap-2">
            <div class="flex flex-col">
              {props.version.title}
              <div class="flex gap-2">
                <div class="m-0 text-shade-3 text-sm flex items-center gap-2">
                  {props.version.modloader} {props.version.mcversion}
                  <div class="h-2 w-px bg-shade-3" /> {props.version.date}
                  <div class="h-2 w-px bg-shade-3" />
                  <span class={getColor(props.version.stable)}>
                    {props.version.stable}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
        <Show when={!props.version.isActive} fallback={<Active />}>
          <div class="group text-shade-3 hover:text-shade-1 transition ease-in-out flex items-center gap-2 cursor-pointer">
            <Trans
              key="switch_version"
              options={{
                defaultValue: "Switch Version",
              }}
            />
            <div class="i-ri:download-2-line text-shade-3 group-hover:text-shade-1 text-2xl" />
          </div>
        </Show>
      </div>
    </div>
  );
};

export default Version;
