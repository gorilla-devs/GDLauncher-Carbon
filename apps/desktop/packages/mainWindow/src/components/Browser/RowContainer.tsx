import { CFFEMod, MRFEProject } from "@gd/core_module/bindings";
import { VersionRowType } from "../InfiniteScrollVersionsQueryWrapper";
import { For, Match, Switch } from "solid-js";
import { Trans } from "@gd/i18n";
import { Spinner } from "@gd/ui";
import { format } from "date-fns";

export type Props = {
  modVersion: VersionRowType;
  project: CFFEMod | MRFEProject | undefined;
  isCurseforge?: boolean;
  instanceId?: number | null;
  installedFile:
    | {
        id: string;
        remoteId: string | number;
      }
    | undefined;
};

export type AdditionalProps = {
  loading: boolean;
  disabled: boolean;
  isInstalled?: boolean;
  onPrimaryAction: () => void;
};

const RowContainer = (props: Props & AdditionalProps) => {
  return (
    <Switch>
      <Match when={props.modVersion}>
        <div class="table-cell py-2 align-middle">
          <h4 class="m-0 font-medium text-md pb-2">
            {props.modVersion.name.replaceAll(".zip", "")}
          </h4>
          <div class="flex text-sm divide-darkSlate-500 text-lightGray-800 divide-x-1 gap-2">
            <Trans key="explore_versions.tags" />
            <For each={props.modVersion.gameVersions}>
              {(version) => <div>{version}</div>}
            </For>
          </div>
        </div>
        <div class="table-cell align-middle">
          {format(new Date(props.modVersion.datePublished), "dd-MM-yyyy")}
        </div>
        <div class="table-cell align-middle">{props.modVersion.downloads}</div>
        <div
          class="table-cell align-middle"
          classList={{
            "text-green-500":
              props.modVersion.releaseType === "stable" ||
              props.modVersion.releaseType === "release",
            "text-yellow-500": props.modVersion.releaseType === "beta",
            "text-red-500": props.modVersion.releaseType === "alpha"
          }}
        >
          {props.modVersion.releaseType}
        </div>
        <div class="table-cell align-middle">
          <div
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer i-ri:information-fill transition-color hover:text-white"
            classList={
              {
                // "text-white": isHoveringInfoCard()
              }
            }
          />
        </div>
        <div
          class="table-cell align-middle"
          classList={{
            "text-green-500": props.isInstalled,
            "text-lightGray-800": !props.disabled && !props.isInstalled,
            "cursor-not-allowed text-lightGray-800": props.disabled
          }}
          onClick={props.onPrimaryAction}
        >
          <div class="flex gap-2">
            <Switch>
              <Match when={!props.instanceId}>
                <Trans key="rowcontainer.no_instance_selected" />
              </Match>
              <Match when={props.loading}>
                <Trans key="modpack.version_downloading" />
                <Spinner class="w-5 h-5" />
              </Match>
              <Match when={!props.loading && !props.isInstalled}>
                <Trans key="modpack.version_download" />
                <div class="i-ri:download-2-line" />
              </Match>
              <Match when={!props.loading && props.isInstalled}>
                <Trans key="modpack.version_installed" />
              </Match>
            </Switch>
          </div>
        </div>
      </Match>
      <Match when={!props.modVersion}>Loading</Match>
    </Switch>
  );
};

export default RowContainer;
