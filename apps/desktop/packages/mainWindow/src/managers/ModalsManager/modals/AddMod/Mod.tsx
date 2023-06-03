import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Spinner, Tag } from "@gd/ui";
import { format } from "date-fns";
import { For, Match, Show, Switch, createSignal } from "solid-js";
import { useModal } from "../..";
import { rspc } from "@/utils/rspcClient";
import { lastInstanceOpened } from "@/utils/routes";

type Props = { mod: FEMod };

const Mod = (props: Props) => {
  const modalsContext = useModal();
  const [loading, setLoading] = createSignal(false);

  const latestFIlesIndexes = () => props.mod.latestFiles;
  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    parseInt(lastInstanceOpened(), 10),
  ]);

  const mappedVersions = () =>
    latestFIlesIndexes().map((version) => ({
      key: version.id,
      label: version.displayName.replaceAll(".jar", ""),
    }));

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    },
    onSettled() {
      setLoading(false);
    },
  });

  const isModInstalled = () =>
    instanceDetails.data?.mods.find(
      (mod) => parseInt(mod.id, 10) === props.mod.id
    ) !== undefined;

  return (
    <div class="flex flex-col gap-4 p-5 bg-darkSlate-700 rounded-2xl max-h-60">
      <div class="flex gap-4">
        <img
          class="select-none rounded-xl h-30 w-30"
          src={props.mod.logo.thumbnailUrl}
        />
        <div class="flex flex-col gap-2">
          <div class="flex flex-col justify-between">
            <h2 class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 max-w-92">
              {props.mod.name}
            </h2>
            <div class="flex gap-4 items-center">
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:time-fill" />
                <div class="whitespace-nowrap text-sm">
                  {format(new Date(props.mod.dateCreated).getTime(), "P")}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:download-fill" />
                <div class="text-sm whitespace-nowrap">
                  {formatDownloadCount(props.mod.downloadCount)}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:user-fill" />
                <div class="text-sm whitespace-nowrap flex gap-2 max-w-52 overflow-x-auto">
                  <For each={props.mod.authors}>
                    {(author) => <p class="m-0">{author.name}</p>}
                  </For>
                </div>
              </div>
            </div>
          </div>
          <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-h-15">
            {truncateText(props.mod?.summary, 137)}
          </p>
        </div>
      </div>
      <div class="flex justify-between items-center gap-3">
        <div class="flex gap-2 overflow-x-auto max-w-100 scrollbar-hide">
          <For each={props.mod.categories}>
            {(tag) => <Tag name={tag.name} img={tag.iconUrl} type="fixed" />}
          </For>
        </div>
        <div class="flex gap-3">
          <Button
            type="outline"
            onClick={() =>
              modalsContext?.openModal(
                {
                  name: "modDetails",
                },
                { mod: props.mod }
              )
            }
          >
            <Trans
              key="instance.explore_modpack"
              options={{
                defaultValue: "Explore",
              }}
            />
          </Button>
          <Switch>
            <Match when={!isModInstalled()}>
              <Dropdown.button
                disabled={loading()}
                options={mappedVersions()}
                rounded
                value={mappedVersions()[0]?.key}
                onClick={() => {
                  installModMutation.mutate({
                    file_id: props.mod.mainFileId,
                    instance_id: parseInt(lastInstanceOpened(), 10),
                    project_id: props.mod.id,
                  });
                }}
                onChange={(val) => {
                  const file = props.mod.latestFiles.find(
                    (file) => file.id === parseInt(val, 10)
                  );
                  if (file) {
                    installModMutation.mutate({
                      file_id: file.id,
                      instance_id: parseInt(lastInstanceOpened(), 10),
                      project_id: props.mod.id,
                    });
                  }
                }}
              >
                <Show when={loading()}>
                  <Spinner />
                </Show>
                <Show when={!loading()}>
                  <Trans
                    key="instance.download_modpacks"
                    options={{
                      defaultValue: "Download",
                    }}
                  />
                </Show>
              </Dropdown.button>
            </Match>
            <Match when={isModInstalled()}>
              <Button>
                <Trans
                  key="mod.downloaded"
                  options={{
                    defaultValue: "Downloaded",
                  }}
                />
              </Button>
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
};

export default Mod;
