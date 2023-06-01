import { Button } from "@gd/ui";
import { For, Show, createEffect, createSignal } from "solid-js";
import { Trans } from "@gd/i18n";
import Version from "./Version";
import glassBlock from "/assets/images/icons/glassBlock.png";
import { useRouteData } from "@solidjs/router";
import fetchData from "../instance.data";
import { rspc } from "@/utils/rspcClient";
import {
  FEFile,
  FEFileIndex,
  InstanceDetails,
  Modpack,
} from "@gd/core_module/bindings";

const NoVersions = () => {
  return (
    <div class="h-full min-h-90 w-full flex justify-center items-center">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.no_resource_packs_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
            }}
          />
        </p>
        <Button type="outline" size="medium">
          <Trans
            key="instance.add_pack"
            options={{
              defaultValue: "+ Add pack",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

const Versions = () => {
  const [versions, setVersions] = createSignal<FEFile[]>([]);
  const [mainFileId, setMainFileId] = createSignal<undefined | number>(
    undefined
  );
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const modId = () =>
    ((routeData.instanceDetails.data as InstanceDetails).modpack as Modpack)
      .Curseforge.project_id;

  if (routeData.instanceDetails.data?.modpack?.Curseforge.project_id) {
    const instanceDetails = rspc.createQuery(() => [
      "modplatforms.curseforgeGetMod",
      {
        modId: modId(),
      },
    ]);

    createEffect(() => {
      console.log("TEST", instanceDetails.data?.data);
      setMainFileId(instanceDetails.data?.data.mainFileId);
      if (instanceDetails.data?.data.latestFilesIndexes) {
        instanceDetails.data?.data.latestFiles.forEach((latestFile) => {
          setVersions((prev) => [...prev, latestFile]);
        });
      }
    });
  }

  return (
    <div>
      <div class="h-full overflow-y-hidden">
        <Show
          when={versions().length > 0 && mainFileId()}
          fallback={<NoVersions />}
        >
          <For each={versions()}>
            {(props) => (
              <Version version={props} mainFileId={mainFileId() as number} />
            )}
          </For>
        </Show>
      </div>
    </div>
  );
};

export default Versions;
