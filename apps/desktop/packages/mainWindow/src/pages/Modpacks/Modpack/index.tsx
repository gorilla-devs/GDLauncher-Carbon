import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { rspc } from "@/utils/rspcClient";
import { FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Tag, createNotification } from "@gd/ui";
import { format } from "date-fns";
import { For, createSignal } from "solid-js";

type Props = { modpack: FEMod };

const Modpack = (props: Props) => {
  const navigate = useGDNavigate();

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);
  const addNotification = createNotification();

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        addNotification("Instance saccessfully created.");
      },
      onError() {
        addNotification("Error while creating the instance.", "error");
      },
    }
  );

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onMutate() {},
      onSuccess(instanceId) {
        prepareInstanceMutation.mutate(instanceId);
      },
      onError() {
        addNotification("Error while downloading the modpack.", "error");
      },
    }
  );
  const latestFIlesIndexes = () => props.modpack.latestFilesIndexes;

  const mappedVersions = () =>
    latestFIlesIndexes().map((version) => ({
      key: version.gameVersion,
      label: version.gameVersion,
    }));

  return (
    <div class="flex flex-col gap-4 p-5 bg-darkSlate-700 rounded-2xl max-h-60">
      <div class="flex gap-4">
        <img
          class="rounded-xl select-none h-30 w-30"
          src={props.modpack.logo.thumbnailUrl}
        />
        <div class="flex flex-col gap-2">
          <div class="flex flex-col justify-between">
            <h2 class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 max-w-92">
              {props.modpack.name}
            </h2>
            <div class="flex gap-4 items-center">
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:time-fill" />
                <div class="whitespace-nowrap text-sm">
                  {format(new Date(props.modpack.dateCreated).getTime(), "P")}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:download-fill" />
                <div class="text-sm whitespace-nowrap">
                  {formatDownloadCount(props.modpack.downloadCount)}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:user-fill" />
                <div class="text-sm whitespace-nowrap flex gap-2 max-w-52 overflow-x-auto">
                  <For each={props.modpack.authors}>
                    {(author) => <p class="m-0">{author.name}</p>}
                  </For>
                </div>
              </div>
            </div>
          </div>
          <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-h-15">
            {truncateText(props.modpack?.summary, 137)}
          </p>
        </div>
      </div>
      <div class="flex justify-between items-center gap-3">
        <div class="flex gap-2 overflow-x-auto max-w-100 scrollbar-hide">
          <For each={props.modpack.categories}>
            {(tag) => <Tag name={tag.name} img={tag.iconUrl} type="fixed" />}
          </For>
        </div>
        <div class="flex gap-3">
          <Button
            type="outline"
            onClick={() => navigate(`/modpacks/${props.modpack.id}`)}
          >
            <Trans
              key="instance.explore_modpack"
              options={{
                defaultValue: "Explore",
              }}
            />
          </Button>
          <Dropdown.button
            options={mappedVersions()}
            rounded
            value={mappedVersions()[0].key}
            onClick={() => {
              loadIconMutation.mutate(props.modpack.logo.url);
              createInstanceMutation.mutate({
                group: defaultGroup.data || 1,
                use_loaded_icon: true,
                notes: "",
                name: props.modpack.name,
                version: {
                  Modpack: {
                    Curseforge: {
                      file_id: props.modpack.mainFileId,
                      project_id: props.modpack.id,
                    },
                  },
                },
              });
            }}
          >
            <Trans
              key="instance.download_modpacks"
              options={{
                defaultValue: "Download",
              }}
            />
          </Dropdown.button>
        </div>
      </div>
    </div>
  );
};

export default Modpack;
