import { CFFEMod, MRFEProject } from "@gd/core_module/bindings";
import { VersionRowTypeData } from "../InfiniteScrollVersionsQueryWrapper";
import { For, Match, Switch } from "solid-js";
import { Trans } from "@gd/i18n";
import { Button, Spinner } from "@gd/ui";
import { format } from "date-fns";

export type Props = {
  modVersion: VersionRowTypeData;
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
        <div class="py-2 flex flex-col justify-center">
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
        <div class="flex items-center">
          {format(new Date(props.modVersion.datePublished), "dd-MM-yyyy")}
        </div>
        <div class="flex items-center">{props.modVersion.downloads}</div>
        <div
          class="flex items-center"
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
        <div class="flex items-center">
          <div
            class="text-2xl text-darkSlate-500 duration-100 ease-in-out cursor-pointer i-ri:information-fill transition-color hover:text-white"
            classList={
              {
                // "text-white": isHoveringInfoCard()
              }
            }
          />
        </div>
        <div class="flex items-center">
          <Button
            type="primary"
            rounded={false}
            disabled={props.disabled || props.isInstalled}
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
                  <Switch>
                    <Match when={props.installedFile}>
                      <Trans key="modpack.version_switch" />
                    </Match>
                    <Match when={!props.installedFile}>
                      <Trans key="modpack.version_download" />
                    </Match>
                  </Switch>
                  <div class="i-ri:download-2-fill" />
                </Match>
                <Match when={!props.loading && props.isInstalled}>
                  <Trans key="modpack.version_installed" />
                </Match>
              </Switch>
            </div>
          </Button>
        </div>
      </Match>
      <Match when={!props.modVersion}>Loading</Match>
    </Switch>
  );
};

export default RowContainer;
