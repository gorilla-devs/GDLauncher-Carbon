import { For, JSX, Match, Suspense, Switch, createEffect } from "solid-js";
import { Skeleton } from "@gd/ui";
import { Props as RowContainerProps } from "@/components/Browser/RowContainer";
import { CFFEMod, MRFEProject } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { VersionRowType } from "../InfiniteScrollVersionsQueryWrapper";

type Props = {
  virtualVersions: any;
  versions: VersionRowType[];
  totalVirtualHeight: number;
  setVirtualListRef: (_el: HTMLDivElement) => void;
  curseforgeProjectData: CFFEMod | undefined;
  modrinthProjectData: MRFEProject | undefined;
  instanceId?: number;
  installedMod?: { id: string; remoteId: string };
  isCurseforge: boolean;
  isLoading: boolean;
  children: (_: RowContainerProps) => JSX.Element;
};

const MainContainer = (props: Props) => {
  return (
    <Suspense fallback={<Skeleton.modpackVersionList />}>
      <div class="table table-auto w-full">
        <div class="table-header-group">
          <div class="table-row h-10">
            <div class="table-cell">
              <Trans key="browser_table_headers.name" />
            </div>
            <div class="table-cell">
              <Trans key="browser_table_headers.published" />
            </div>
            <div class="table-cell">
              <Trans key="browser_table_headers.downloads" />
            </div>
            <div class="table-cell">
              <Trans key="browser_table_headers.type" />
            </div>
            <div class="table-cell">
              <Trans key="browser_table_headers.details" />
            </div>
          </div>
        </div>
        <div
          class="pt-10 relative"
          ref={(el) => {
            props.setVirtualListRef(el);
          }}
          style={{
            height: `${props.totalVirtualHeight}px`
          }}
        >
          <Switch fallback={<Skeleton.modpackVersionList />}>
            <Match when={props.versions?.length > 0}>
              <For each={props.virtualVersions}>
                {(modFile: any) => {
                  return (
                    <div
                      data-index={modFile.index}
                      class="table-row rounded-md hover:bg-darkSlate-700"
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        height: `${modFile.size}px`,
                        transform: `translateY(${modFile.start}px)`
                      }}
                    >
                      <props.children
                        project={
                          props.curseforgeProjectData ||
                          props.modrinthProjectData
                        }
                        isCurseforge={props.isCurseforge}
                        installedFile={props.installedMod}
                        modVersion={props.versions[modFile.index]}
                        instanceId={props.instanceId}
                      />
                    </div>
                  );
                }}
              </For>
            </Match>
            <Match when={props.isLoading}>
              <Skeleton.modpackVersionList />
            </Match>
          </Switch>
        </div>
      </div>
    </Suspense>
  );
};

export default MainContainer;
