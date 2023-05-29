import { FEFile } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { format } from "date-fns";

type Props = {
  modVersion: FEFile;
};

const VersionRow = (props: Props) => {
  return (
    <div class="flex flex-col py-2">
      <h4 class="font-medium m-0">{props.modVersion.displayName}</h4>
      <div class="flex justify-between">
        <div class="flex justify-between">
          <div class="flex justify-between text-sm divide-darkSlate-500 text-lightGray-800 divide-x-1">
            <span class="pr-3">{props.modVersion.gameVersions[0]}</span>
            <span class="px-3">
              {format(new Date(props.modVersion.fileDate), "dd-MM-yyyy")}
            </span>
            <span
              class="pl-3"
              classList={{
                "text-green-500": props.modVersion.releaseType === "stable",
                "text-yellow-500": props.modVersion.releaseType === "beta",
                "text-red-500": props.modVersion.releaseType === "alpha",
              }}
            >
              {props.modVersion.releaseType}
            </span>
          </div>
        </div>
        <span class="flex gap-2 text-lightGray-800 cursor-pointer select-none">
          <Trans
            key="modpack.version_download"
            options={{
              defaultValue: "Download version",
            }}
          />
          <div class="i-ri:download-2-line" />
        </span>
      </div>
    </div>
  );
};

export default VersionRow;
