import { Trans } from "@gd/i18n";
import { Show } from "solid-js";
import { format } from "date-fns";
import { CFFEFile } from "@gd/core_module/bindings";

type Props = {
  version: CFFEFile;
  mainFileId: number;
};

const getColor = (stable: string) => {
  switch (stable) {
    case "stable":
      return "text-green-500";
    case "beta":
      return "text-yellow-500";
    case "alpha":
      return "text-red-500";
    default:
      return "text-green-500";
  }
};

const Active = () => {
  return (
    <div class="flex items-center gap-2 text-green-500">
      <Trans
        key="instance.active_version"
        options={{
          defaultValue: "Active"
        }}
      />
      <div class="text-green-500 text-2xl i-ri:check-fill" />
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
              <p class="mt-0 mb-2">{props.version.displayName}</p>
              <div class="flex gap-2">
                <div class="m-0 text-sm flex items-center gap-2 text-darkSlate-300">
                  {props.version.gameVersions[1]}{" "}
                  {props.version.gameVersions[0]}
                  <div class="h-2 w-px bg-darkSlate-300" />
                  <p class="m-0 text-darkSlate-300 text-md">
                    {format(new Date(props.version.fileDate), "dd-MM-yyyy")}
                  </p>
                  <div class="h-2 w-px bg-darkSlate-300" />
                  <span class={getColor(props.version.releaseType)}>
                    {props.version.releaseType}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
        <Show
          when={props.mainFileId === props.version.id}
          fallback={<Active />}
        >
          <div class="group text-darkSlate-300 transition ease-in-out flex items-center gap-2 cursor-pointer hover:text-darkSlate-100">
            <Trans
              key="instance.switch_version"
              options={{
                defaultValue: "Switch Version"
              }}
            />
            <div class="text-darkSlate-300 text-2xl i-ri:download-2-line group-hover:text-darkSlate-100" />
          </div>
        </Show>
      </div>
    </div>
  );
};

export default Version;
