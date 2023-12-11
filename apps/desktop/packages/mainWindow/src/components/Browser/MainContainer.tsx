import { For, JSX, Match, Suspense, Switch } from "solid-js";
import { Skeleton } from "@gd/ui";
import { Props as RowContainerProps } from "@/components/Browser/RowContainer";
import {
  CFFEFile,
  CFFEMod,
  MRFEProject,
  MRFEVersion
} from "@gd/core_module/bindings";

type Props = {
  versions: CFFEFile[] | MRFEVersion[];
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
      <div class="flex flex-col">
        <Switch fallback={<Skeleton.modpackVersionList />}>
          <Match when={props.versions?.length > 0}>
            <For each={props.versions}>
              {(modFile) => (
                <props.children
                  project={
                    props.curseforgeProjectData || props.modrinthProjectData
                  }
                  isCurseforge={props.isCurseforge}
                  installedFile={props.installedMod}
                  modVersion={modFile}
                  instanceId={props.instanceId}
                />
              )}
            </For>
          </Match>
          <Match when={props.isLoading}>
            <Skeleton.modpackVersionList />
          </Match>
        </Switch>
      </div>
    </Suspense>
  );
};

export default MainContainer;
