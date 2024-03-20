import { Button } from "@gd/ui";
import { For, Show, createEffect, createSignal } from "solid-js";
import { Trans } from "@gd/i18n";
import Version from "./Version";
import skull from "/assets/images/icons/skull.png";
import { useRouteData } from "@solidjs/router";
import fetchData from "../../instance.data";
import { rspc } from "@/utils/rspcClient";
import { CFFEFile } from "@gd/core_module/bindings";

const NoVersions = () => {
  return (
    <div class="h-full min-h-90 w-full flex justify-center items-center">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={skull} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="modpack.no_versions_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain any other versions"
            }}
          />
        </p>
        <Button type="outline" size="medium">
          <Trans
            key="modpack.no_versions"
            options={{
              defaultValue: "No versions"
            }}
          />
        </Button>
      </div>
    </div>
  );
};

const Versions = () => {
  const [versions, setVersions] = createSignal<CFFEFile[]>([]);
  const [mainFileId, setMainFileId] = createSignal<undefined | number>(
    undefined
  );
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const modId = () =>
    routeData.instanceDetails?.data?.modpack?.modpack?.type === "curseforge" &&
    routeData.instanceDetails.data?.modpack?.modpack.value?.project_id;

  if (modId()) {
    const instanceDetails = rspc.createQuery(() => ({
      queryKey: [
        "modplatforms.curseforge.getMod",
        {
          modId: modId() as number
        }
      ]
    }));

    createEffect(() => {
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
